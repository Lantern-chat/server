use std::ops::Deref;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_postgres::{AsyncMessage, Connection as StdConnection, Error};

/// Simple wrapper type for `tokio_postgres::Connection` that returns the actual message in the future
pub struct ConnectionStream<S, T>(pub StdConnection<S, T>);

impl<S, T> Deref for ConnectionStream<S, T> {
    type Target = StdConnection<S, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S, T> Stream for ConnectionStream<S, T>
where
    S: AsyncRead + AsyncWrite + Unpin,
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<AsyncMessage, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_message(cx)
    }
}
