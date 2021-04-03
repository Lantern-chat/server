use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tokio::sync::mpsc::{self, error::TrySendError};

pub struct LaggyReceiver<T> {
    lagged: Arc<AtomicBool>,
    rx: mpsc::Receiver<T>,
}

pub struct LaggySender<T> {
    lagged: Arc<AtomicBool>,
    tx: mpsc::Sender<T>,
}

impl<T> Clone for LaggySender<T> {
    fn clone(&self) -> Self {
        LaggySender {
            lagged: self.lagged.clone(),
            tx: self.tx.clone(),
        }
    }
}

pub fn channel<T>(size: usize) -> (LaggySender<T>, LaggyReceiver<T>) {
    let (tx, rx) = mpsc::channel(size);
    let lagged = Arc::new(AtomicBool::new(false));

    (
        LaggySender {
            lagged: lagged.clone(),
            tx,
        },
        LaggyReceiver { lagged, rx },
    )
}

#[derive(Debug, thiserror::Error)]
pub enum LaggyRecvError {
    #[error("Lagged")]
    Lagged,
}

impl<T> LaggySender<T> {
    pub fn try_send(&self, message: T) -> Result<(), TrySendError<T>> {
        match self.tx.try_send(message) {
            Ok(()) => Ok(()),
            Err(e) => {
                if let TrySendError::Full(_) = e {
                    self.lagged.store(true, Ordering::SeqCst);
                }
                Err(e)
            }
        }
    }
}

impl<T> LaggyReceiver<T> {
    /// If this returns Err, then the channel lagged and should be terminated
    pub async fn recv(&mut self) -> Result<Option<T>, LaggyRecvError> {
        if self.lagged.load(Ordering::SeqCst) {
            return Err(LaggyRecvError::Lagged);
        }

        Ok(self.rx.recv().await)
    }
}
