//! Task Runner
//!
//! This small crate defines a simple structure for collecting and running tasks, which are simply
//! defined as a high-level operations or loops that run independently of
//! each other (albiet likely on shared state)

use futures::TryStreamExt;
use std::future::Future;
use std::sync::Arc;
use tokio::task::JoinError;
use tokio::{sync::watch, task::JoinHandle};

use futures::stream::{FuturesUnordered, Stream, StreamExt};

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

pub fn fn_task<S, T, F>(state: S, f: T) -> impl Task
where
    T: FnOnce(watch::Receiver<bool>, S) -> F + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
    S: Send + 'static,
{
    struct AsyncTask<S, T>(S, T);

    impl<S, T, F> Task for AsyncTask<S, T>
    where
        T: FnOnce(watch::Receiver<bool>, S) -> F + Send + 'static,
        F: Future<Output = ()> + Send + 'static,
        S: Send + 'static,
    {
        fn start(self, alive: watch::Receiver<bool>) -> JoinHandle<()> {
            tokio::task::spawn(async move {
                let AsyncTask(state, f) = self;
                f(alive, state).await
            })
        }
    }

    AsyncTask(state, f)
}

use tokio::time::Duration;

pub fn interval_fn_task<S, T, F>(state: S, interval: Duration, f: T) -> impl Task
where
    T: Fn(tokio::time::Instant, &S) -> F + Send + Sync + 'static,
    F: Future<Output = ()> + Send + 'static,
    S: Send + Sync + 'static,
{
    fn_task(state, move |mut alive, state| async move {
        let mut interval = tokio::time::interval(interval);

        while *alive.borrow_and_update() {
            tokio::select! {
                biased;
                t = interval.tick() => f(t, &state).await,
                _ = alive.changed() => break,
            }
        }
    })
}
