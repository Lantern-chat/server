pub mod embed;
pub mod strip;

use std::sync::Arc;
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        OwnedSemaphorePermit, Semaphore, SemaphorePermit,
    },
    task::JoinHandle,
};

pub trait WorkItem: Send + 'static {
    fn run(self) -> JoinHandle<()>;
}

pub struct Queue<W> {
    pub tx: Sender<(W, OwnedSemaphorePermit)>,
    pub limit: Arc<Semaphore>,
}

impl<W: WorkItem> Queue<W> {
    pub fn start(buffer: usize) -> Self {
        let len = buffer * num_cpus::get();

        let (tx, mut rx) = channel(len);
        let limit = Arc::new(Semaphore::new(len));

        tokio::spawn(async move {
            while let Some((work, permit)) = rx.recv().await {

                // do stuff
            }
        });

        Queue { tx, limit }
    }
}
