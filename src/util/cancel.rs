use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use futures::{Stream, StreamExt};

#[pin_project::pin_project]
pub struct CancelableStream<S> {
    canceled: Arc<AtomicBool>,
    #[pin]
    inner: S,
}

pub struct Cancel {
    canceled: Arc<AtomicBool>,
}

impl Cancel {
    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::SeqCst);
    }
}

impl<S> CancelableStream<S> {
    pub fn new(inner: S) -> (Self, Cancel) {
        let canceled = Arc::new(AtomicBool::new(false));

        let stream = CancelableStream {
            canceled: canceled.clone(),
            inner,
        };

        (stream, Cancel { canceled })
    }
}

use std::pin::Pin;
use std::task::{Context, Poll};

impl<S> Stream for CancelableStream<S>
where
    S: Stream,
{
    type Item = <S as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.canceled.load(Ordering::SeqCst) {
            return Poll::Ready(None);
        }

        self.project().inner.poll_next(cx)
    }
}
