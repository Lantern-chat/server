use futures::Future;
use std::sync::Arc;
use tokio::{
    sync::{AcquireError, OwnedSemaphorePermit, Semaphore, SemaphorePermit},
    task::JoinHandle,
};

pub trait WorkItem: Send + 'static {
    fn run(self) -> JoinHandle<()>;
}

pub struct Queue {
    pub limit: Arc<Semaphore>,
}

impl Queue {
    pub fn stop(&self) {
        self.limit.close();
    }

    pub fn start(per_thread_buffer: usize) -> Self {
        Queue {
            limit: Arc::new(Semaphore::new(per_thread_buffer * num_cpus::get())),
        }
    }

    pub fn try_push<W: WorkItem>(&self, work: W) -> Result<JoinHandle<()>, W> {
        let Ok(permit) = self.limit.clone().try_acquire_owned() else { return Err(work) };

        Ok(tokio::spawn(async move {
            if let Err(e) = work.run().await {
                log::error!("Error running queue work item: {e}");
            }

            drop(permit);
        }))
    }

    pub fn push<W: WorkItem>(&self, work: W) -> JoinHandle<Result<(), W>> {
        let limit = self.limit.clone();

        tokio::spawn(async move {
            match limit.clone().acquire_owned().await {
                Ok(permit) => {
                    if let Err(e) = work.run().await {
                        log::error!("Error running queue work item: {e}");
                    }

                    drop(permit);

                    Ok(())
                }
                Err(_) => Err(work),
            }
        })
    }

    pub fn grow(&self, permits: usize) {
        self.limit.add_permits(permits);
    }

    pub async fn shrink(&self, by: u32) -> Result<(), AcquireError> {
        let permits = self.limit.acquire_many(by).await?;

        permits.forget();

        Ok(())
    }
}

#[repr(transparent)]
pub struct WorkFunction<F>(pub F);

impl<F, R> WorkItem for WorkFunction<F>
where
    F: Send + FnOnce() -> R + 'static,
    R: Send + Future<Output = ()> + 'static,
{
    #[inline]
    fn run(self) -> JoinHandle<()> {
        tokio::spawn(self.0())
    }
}
