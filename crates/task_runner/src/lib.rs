//! Task Runner
//!
//! This small crate defines a simple structure for collecting and running tasks, which are simply
//! defined as a high-level operations or loops that run independently of
//! each other (albiet likely on shared state)

extern crate tracing as log;

use std::future::Future;
use std::sync::Arc;
use tokio::task::JoinError;
use tokio::{sync::watch, task::JoinHandle};

use futures::stream::{FuturesUnordered, Stream, StreamExt, TryStreamExt};

pub trait Task {
    fn start(self, alive: watch::Receiver<bool>) -> JoinHandle<()>;
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ShutdownSignal(Arc<watch::Sender<bool>>);

impl ShutdownSignal {
    fn new() -> Self {
        ShutdownSignal(Arc::new(watch::channel(true).0))
    }

    fn subscribe(&self) -> watch::Receiver<bool> {
        self.0.subscribe()
    }

    pub fn stop(&self) {
        self.0.send(false).expect("Could not stop task runner!");
    }
}

pub struct TaskRunner {
    tasks: FuturesUnordered<JoinHandle<()>>,
    alive: ShutdownSignal,
}

impl TaskRunner {
    pub fn new() -> Self {
        TaskRunner {
            tasks: FuturesUnordered::new(),
            alive: ShutdownSignal::new(),
        }
    }

    pub fn add(&self, task: impl Task) {
        self.tasks.push(task.start(self.alive.subscribe()))
    }

    pub fn stop(&self) {
        self.alive.stop();
    }

    pub fn signal(&self) -> ShutdownSignal {
        self.alive.clone()
    }

    pub async fn wait(self) -> Result<(), JoinError> {
        self.try_fold((), |_, _| futures::future::ok(())).await
    }
}

impl Stream for TaskRunner {
    type Item = Result<(), JoinError>;

    #[inline]
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.tasks.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.tasks.size_hint()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsyncFnTask<T>(pub T);

impl<T, F> AsyncFnTask<T>
where
    T: FnOnce(watch::Receiver<bool>) -> F + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    pub const fn new(f: T) -> Self {
        AsyncFnTask(f)
    }
}

impl<T, F> Task for AsyncFnTask<T>
where
    T: FnOnce(watch::Receiver<bool>) -> F + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    fn start(self, alive: watch::Receiver<bool>) -> JoinHandle<()> {
        // NOTE: Call task within async block to defer initial execution
        tokio::task::spawn(async move { (self.0)(alive).await })
    }
}

use tokio::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalFnTask<T, S>(pub S, pub Duration, pub T);

impl<T, F, S> IntervalFnTask<T, S>
where
    T: Fn(S, Instant, &watch::Receiver<bool>) -> F + Send + Sync + 'static,
    F: Future<Output = ()> + Send + 'static,
    S: Clone + Send + Sync + 'static,
{
    pub const fn new(state: S, interval: Duration, f: T) -> Self {
        IntervalFnTask(state, interval, f)
    }
}

impl<T, F, S> Task for IntervalFnTask<T, S>
where
    T: Fn(S, Instant, &watch::Receiver<bool>) -> F + Send + Sync + 'static,
    F: Future<Output = ()> + Send + 'static,
    S: Clone + Send + Sync + 'static,
{
    fn start(self, alive: watch::Receiver<bool>) -> JoinHandle<()> {
        AsyncFnTask(move |mut alive: watch::Receiver<bool>| async move {
            let IntervalFnTask(state, interval, f) = self;

            let mut interval = tokio::time::interval(interval);

            while *alive.borrow_and_update() {
                tokio::select! {
                    biased;
                    t = interval.tick() => f(state.clone(), t, &alive).await,
                    _ = alive.changed() => break,
                }
            }
        })
        .start(alive)
    }
}

use failsafe::futures::CircuitBreaker;
use failsafe::{Config, Error as Reject, FailurePolicy, Instrument};

#[derive(Debug)]
pub struct RetryTask<T, POLICY, INSTRUMENT>(pub Config<POLICY, INSTRUMENT>, pub T);

impl<T: Task + Clone> RetryTask<T, (), ()>
where
    T: Send + Sync + 'static,
{
    pub fn new(task: T) -> impl Task {
        RetryTask(Config::new(), task)
    }
}

impl<T: Task + Clone, POLICY: FailurePolicy, INSTRUMENT: Instrument> Task for RetryTask<T, POLICY, INSTRUMENT>
where
    T: Send + Sync + 'static,
    POLICY: Send + Sync + 'static,
    INSTRUMENT: Send + Sync + 'static,
{
    fn start(self, mut alive: watch::Receiver<bool>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let RetryTask(config, task) = self;

            let cb = config.build();

            while *alive.borrow_and_update() {
                match cb.call(async { task.clone().start(alive.clone()).await }).await {
                    Ok(()) => log::trace!("Task ran successfully"),
                    Err(Reject::Inner(e)) => log::error!("Error running task: {e}"),
                    Err(Reject::Rejected) => {
                        log::warn!("Task has been rate-limited");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        })
    }
}
