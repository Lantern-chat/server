use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_postgres::{AsyncMessage, Connection as StdConnection, Error};

/// Simple wrapper type for `tokio_postgres::Connection` that returns the actual message in the future
pub struct Connection<S, T>(StdConnection<S, T>);

impl<S, T> Deref for Connection<S, T> {
    type Target = StdConnection<S, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum DbMessage {
    Shutdown,
    Message(AsyncMessage),
    Err(Error),
}

impl<S, T> Future for Connection<S, T>
where
    S: AsyncRead + AsyncWrite + Unpin,
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Output = DbMessage;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0.poll_message(cx) {
            Poll::Ready(res) => Poll::Ready(match res {
                None => DbMessage::Shutdown,
                Some(Ok(msg)) => DbMessage::Message(msg),
                Some(Err(err)) => DbMessage::Err(err),
            }),
            _ => Poll::Pending,
        }
    }
}
