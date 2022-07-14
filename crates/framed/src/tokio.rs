use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter, ReadBuf};

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

    /// Use message within a callback and have it be closed automatically after
    pub async fn with_msg<F, R, T, E>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut BufWriter<AsyncMessageWriter<W>>) -> R,
        R: Future<Output = Result<T, E>>,
        E: From<io::Error>,
    {
        let mut msg = self.new_message();
        match f(&mut msg).await {
            Ok(t) => match msg.flush().await {
                Ok(()) => match msg.into_inner().close().await {
                    Ok(()) => Ok(t),
                    Err(e) => Err(e.into()),
                },
                Err(e) => Err(io::Error::from(e).into()),
            },
            Err(e) => Err(e),
        }
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

pub struct AsyncFramedReader<R: AsyncRead> {
    inner: R,
    msg: u64,
    len: u64,

    header: [u8; 16],
    pos: usize,
}

impl<R: AsyncRead + Unpin> AsyncFramedReader<R> {
    pub fn new(inner: R) -> Self {
        AsyncFramedReader {
            inner,
            msg: 0,
            len: 0,
            header: [0; 16],
            pos: 0,
        }
    }

    pub async fn next_msg<'a>(&'a mut self) -> io::Result<Option<&'a mut Self>> {
        if self.len > 0 {
            io::copy(self, &mut io::sink()).await?;
        }

        loop {
            return match self.inner.read_exact(&mut self.header).await {
                Ok(_) => Ok({
                    // ensure the header cursor is at the end
                    self.pos = self.header.len();

                    let mut msg = [0u8; 8];
                    let mut len = [0u8; 8];

                    msg.copy_from_slice(&self.header[0..8]);
                    len.copy_from_slice(&self.header[8..16]);

                    self.msg = u64::from_be_bytes(msg);
                    self.len = u64::from_be_bytes(len);

                    if self.len == 0 {
                        continue;
                    }

                    Some(self)
                }),
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
                Err(e) => Err(e),
            };
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for AsyncFramedReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.len == 0 {
            // reset header position if it was previously fully read
            if self.pos == self.header.len() {
                self.pos = 0;
            }

            while self.pos < self.header.len() {
                // split mutable borrows
                let AsyncFramedReader {
                    ref mut header,
                    ref mut pos,
                    ref mut inner,
                    ..
                } = *self.as_mut();

                let mut buf = ReadBuf::new(header);
                buf.set_filled(*pos);

                match Pin::new(inner).poll_read(cx, &mut buf) {
                    Poll::Ready(Ok(())) => {
                        *pos = buf.filled().len();
                    }
                    other => return other,
                }
            }

            let mut msg = [0u8; 8];
            let mut len = [0u8; 8];

            msg.copy_from_slice(&self.header[0..8]);
            len.copy_from_slice(&self.header[8..16]);

            self.msg = u64::from_be_bytes(msg);
            self.len = u64::from_be_bytes(len);

            if self.len == 0 {
                return Poll::Ready(Ok(())); // EOF for end of message
            }
        }

        // automatically takes `min(remaining, len)`
        let mut buf = buf.take(self.len as usize);
        match Pin::new(&mut self.inner).poll_read(cx, &mut buf) {
            Poll::Ready(Ok(())) => {
                self.len -= buf.filled().len() as u64;

                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}
