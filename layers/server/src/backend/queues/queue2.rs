use futures::{future::BoxFuture, stream::FuturesUnordered};
use futures::{FutureExt, StreamExt};
use std::future::Future;
use tokio::sync::mpsc::{self, error::SendError};
use tokio::sync::oneshot;

pub trait WorkItem: Future<Output = ()> + Send + 'static {}

impl<F> WorkItem for F where F: Future<Output = ()> + Send + 'static {}

pub type DynamicQueue = Queue<BoxFuture<'static, ()>>;

type WorkTx<W> = (W, oneshot::Sender<()>);

pub struct Queue<W> {
    tx: mpsc::UnboundedSender<WorkTx<W>>,
}

impl<W: WorkItem> Queue<W> {
    pub fn enqueue(&self, work: W) -> Result<oneshot::Receiver<()>, W> {
        let (txs, rxs) = oneshot::channel();
        match self.tx.send((work, txs)) {
            Ok(_) => Ok(rxs),
            Err(e) => Err((e.0).0),
        }
    }

    pub fn start(limit: usize) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<WorkTx<W>>();

        tokio::spawn(async move {
            let mut pending = FuturesUnordered::new();

            loop {
                // top-priority is finishing existing work
                while pending.len() >= limit {
                    pending.next().await;
                }

                // run these concurrently
                // NOTE: When the server shuts down, tx will close as State drops, breaking the loop
                tokio::select! {
                    biased;
                    _ = pending.next() => continue,
                    work = rx.recv() => match work {
                        Some((work, signal)) => pending.push(work.map(|()| {
                            if let Err(e) = signal.send(()) {
                                log::error!("Error sending signal on queue work end: {e:?}");
                            }
                        })),
                        None => break, // channel closed
                    },
                }
            }

            // finish any remaining work items before exiting
            let _ = pending.count().await;
        });

        Queue { tx }
    }
}
