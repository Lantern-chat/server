use tokio::io::{self, AsyncRead, AsyncWrite, AsyncWriteExt, BufWriter};

pub struct AsyncFramedWriter<W> {
    inner: W,
    msg: u64,
}

impl<W: AsyncWrite + Unpin> AsyncFramedWriter<W> {
    pub fn new(inner: W) -> Self {
        AsyncFramedWriter { inner, msg: 1 }
    }

    pub fn new_message<'a>(&'a mut self) -> BufWriter<AsyncMessageWriter<'a, W>> {
        self.msg += 1;
        BufWriter::new(AsyncMessageWriter {
            header: {
                let mut header = [0; 16];
                header[0..8].copy_from_slice(&self.msg.to_be_bytes());
                header
            },
            w: self,
            len: 0,
            pos: 0,
        })
    }
}

pub struct AsyncMessageWriter<'a, W> {
    w: &'a mut AsyncFramedWriter<W>,

    len: u64,
    header: [u8; 16],
    pos: usize,
}

impl<W: AsyncWrite + Unpin> AsyncMessageWriter<'_, W> {
    #[inline]
    fn inner(&mut self) -> Pin<&mut W> {
        Pin::new(&mut self.w.inner)
    }

    async fn try_close(&mut self) -> io::Result<()> {
        // set length to 0 for closing frame
        self.header[8..16].fill(0);

        self.w.inner.write_all(&self.header).await?;
        self.w.inner.flush().await
    }

    /// Close the message
    pub async fn close(mut self) -> io::Result<()> {
        let res = self.try_close().await;

        std::mem::forget(self);

        res
    }
}

use std::pin::Pin;
use std::task::{Context, Poll};

impl<W> Drop for AsyncMessageWriter<'_, W> {
    fn drop(&mut self) {
        panic!("AsyncMessageWriter cannot be dropped! Use `.close()`!");
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for AsyncMessageWriter<'_, W> {
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.inner().poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.inner().poll_shutdown(cx)
    }

    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }

        if self.len == 0 {
            let len = buf.len() as u64;
            self.len = len;
            self.pos = 0; // reset position so header can be written out
            self.header[8..16].copy_from_slice(&len.to_be_bytes());
        }

        // try to write out the whole header in one call, even if has a partial write.
        // pending is fine since it doesn't return a value for bytes written, hiding the overhead.
        while self.pos < self.header.len() {
            let header = self.header;
            let pos = self.pos;
            match self.inner().poll_write(cx, &header[pos..]) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Ready(Ok(n)) => {
                    self.pos += n;
                }
            }
        }

        let len = self.len;
        let bytes_to_write = buf.len().min(len as usize);
        Poll::Ready(match self.inner().poll_write(cx, &buf[bytes_to_write..]) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(e)) => Err(e),
            Poll::Ready(Ok(bytes_written)) => {
                self.len -= bytes_written as u64;
                Ok(bytes_written)
            }
        })
    }
}
