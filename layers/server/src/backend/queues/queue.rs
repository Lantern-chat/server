use futures::Future;
use std::sync::Arc;
use tokio::{
    sync::{AcquireError, Semaphore},
    task::JoinHandle,
};

pub trait WorkItem: Future<Output = Self::Res> + Send + 'static {
    type Res: Send + 'static;
}

impl<F, R> WorkItem for F
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    type Res = R;
}

#[derive(Debug, Clone)]
pub struct Queue {
    pub limit: Arc<Semaphore>,
}

impl Queue {
    pub fn stop(&self) {
        self.limit.close();
    }

    pub fn start(max_concurrent: usize) -> Self {
        Queue {
            limit: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub fn try_push<W: WorkItem>(&self, work: W) -> Result<JoinHandle<W::Res>, W> {
        let Ok(permit) = self.limit.clone().try_acquire_owned() else {
            return Err(work);
        };

        Ok(tokio::spawn(async move {
            let res = work.await;
            drop(permit);
            res
        }))
    }

    pub fn push<W: WorkItem>(&self, work: W) -> JoinHandle<Result<W::Res, W>> {
        let limit = self.limit.clone();

        tokio::spawn(async move {
            let Ok(permit) = limit.acquire().await else {
                return Err(work);
            };

            let res = work.await;
            drop(permit);
            Ok(res)
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
