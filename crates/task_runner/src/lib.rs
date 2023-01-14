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

use futures::stream::{self, FuturesUnordered, Stream, StreamExt, TryStreamExt};

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

pub trait IntervalStream<S>: Send + 'static {
    type Stream: Stream<Item = Duration> + Send;

    fn interval(self, state: &S) -> Self::Stream;
}

// yields once and then forever pending
impl<S> IntervalStream<S> for Duration {
    type Stream = stream::Chain<stream::Once<futures::future::Ready<Duration>>, stream::Pending<Duration>>;

    fn interval(self, _: &S) -> Self::Stream {
        stream::once(futures::future::ready(self)).chain(stream::pending())
    }
}

impl<F, S, R> IntervalStream<S> for F
where
    F: FnOnce(&S) -> R + Send + 'static,
    R: Stream<Item = Duration> + Send + 'static,
{
    type Stream = R;

    fn interval(self, state: &S) -> Self::Stream {
        (self)(state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalFnTask<T, S, I>(pub S, pub I, pub T);

impl<T, F, S, I> IntervalFnTask<T, S, I>
where
    T: Fn(S, &watch::Receiver<bool>) -> F + Send + Sync + 'static,
    F: Future<Output = ()> + Send + 'static,
    S: Clone + Send + Sync + 'static,
    I: IntervalStream<S>,
{
    pub const fn new(state: S, interval: I, f: T) -> Self {
        IntervalFnTask(state, interval, f)
    }
}

fn clean_interval(mut interval: Duration) -> Duration {
    if interval.is_zero() {
        // 100 years
        interval = Duration::from_secs(60 * 60 * 24 * 365 * 100);
    }
    interval
}

impl<T, F, S, I> Task for IntervalFnTask<T, S, I>
where
    T: Fn(S, &watch::Receiver<bool>) -> F + Send + Sync + 'static,
    F: Future<Output = ()> + Send + 'static,
    S: Clone + Send + Sync + 'static,
    I: IntervalStream<S>,
{
    fn start(self, alive: watch::Receiver<bool>) -> JoinHandle<()> {
        AsyncFnTask(move |mut alive: watch::Receiver<bool>| async move {
            let IntervalFnTask(state, i, f) = self;

            let interval = i.interval(&state);
            futures::pin_mut!(interval);

            let mut current_interval = interval.next().await.unwrap_or_default();

            let sleep = tokio::time::sleep(clean_interval(current_interval));
            futures::pin_mut!(sleep);

            while *alive.borrow_and_update() {
                let mut deadline = sleep.deadline();

                tokio::select! {
                    biased;
                    _ = &mut sleep => {
                        // never run if zero, even after 100 years
                        if !current_interval.is_zero() {
                            f(state.clone(), &alive).await
                        }
                    },
                    _ = alive.changed() => break,

                    // if the interval value changes, we are almost certainly *before* the deadline
                    i = interval.next() => match i {
                        // TODO: Revisit this logic to double-check
                        Some(new_interval) => {
                            let previous_deadline = deadline - clean_interval(current_interval);

                            // new_interval is explicitely not cleaned
                            let next_deadline = previous_deadline + new_interval;

                            // if the time between runs is being reduced and the task is expected
                            // to run sooner (next_deadline < deadline), compute how much time is left
                            // before it needs to be run (diff)
                            deadline = match deadline.checked_duration_since(next_deadline) {
                                None => previous_deadline,
                                Some(diff) => {
                                    // set the now-previous deadline to a value such that the task will run at
                                    // most once (now) with the shorter interval, or after
                                    // new_interval from the previous deadline
                                    Instant::now() - diff.min(new_interval)
                                }
                            };

                            current_interval = new_interval;

                        }
                        None => {}
                    }
                }

                // reset to next deadline
                sleep.as_mut().reset(deadline + clean_interval(current_interval));
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

pub struct RetryAsyncFnTask<T, S, POLICY, INSTRUMENT>(pub Config<POLICY, INSTRUMENT>, pub T, pub S);

impl<T, F, E, S> RetryAsyncFnTask<T, S, (), ()>
where
    T: Fn(watch::Receiver<bool>, S) -> F + Send + 'static,
    for<'a> &'a T: Send,
    F: Future<Output = Result<(), E>> + Send + 'static,
    E: std::error::Error + Send + 'static,
    S: Clone + Send + 'static,
{
    pub fn new(state: S, f: T) -> impl Task {
        RetryAsyncFnTask(Config::new(), f, state)
    }
}

impl<T, F, E, S, POLICY: FailurePolicy, INSTRUMENT: Instrument> Task for RetryAsyncFnTask<T, S, POLICY, INSTRUMENT>
where
    T: Fn(watch::Receiver<bool>, S) -> F + Send + 'static,
    for<'a> &'a T: Send,
    F: Future<Output = Result<(), E>> + Send + 'static,
    E: std::error::Error + Send + 'static,
    S: Clone + Send + 'static,
    POLICY: Send + Sync + 'static,
    INSTRUMENT: Send + Sync + 'static,
{
    fn start(self, alive: watch::Receiver<bool>) -> JoinHandle<()> {
        AsyncFnTask::new(|mut alive| async move {
            let RetryAsyncFnTask(config, task, state) = self;

            let cb = config.build();

            while *alive.borrow_and_update() {
                // avoid &S: Send bounds by cloning ahead of time
                let state = state.clone();

                match cb.call(async { task(alive.clone(), state).await }).await {
                    Ok(()) => log::trace!("Task ran successfully"),
                    Err(Reject::Inner(e)) => log::error!("Error running task: {e}"),
                    Err(Reject::Rejected) => {
                        log::warn!("Task has been rate-limited");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        })
        .start(alive)
    }
}
