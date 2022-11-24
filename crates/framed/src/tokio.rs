use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter, ReadBuf};

pub struct AsyncFramedWriter<W> {
    inner: W,
}

// Leaving here for later...
// pub trait WithMsgCallback<'a, W, T, E>:
//     FnOnce(&'a mut BufWriter<AsyncMessageWriter<W>>) -> Self::Fut + 'a
// {
//     type Fut: Future<Output = Result<T, E>> + 'a;
// }

// impl<'a, F, FF, W, T, E> WithMsgCallback<'a, W, T, E> for F
// where
//     F: FnOnce(&'a mut BufWriter<AsyncMessageWriter<W>>) -> FF + 'a,
//     FF: Future<Output = Result<T, E>> + 'a,
// {
//     type Fut = FF;
// }

impl<W: AsyncWrite + Unpin> AsyncFramedWriter<W> {
    pub fn new(inner: W) -> Self {
        AsyncFramedWriter { inner }
    }

    pub fn new_message<'a>(&'a mut self) -> BufWriter<AsyncMessageWriter<'a, W>> {
        BufWriter::new(AsyncMessageWriter {
            w: self,
            len: 0,
            header: [0; 8],
            pos: 0,
        })
    }

    pub async fn dispose_msg(mut msg: BufWriter<AsyncMessageWriter<'_, W>>) -> io::Result<()> {
        match msg.flush().await {
            Ok(()) => msg.into_inner().close().await,
            Err(e) => {
                // don't let AsyncMessageWriter drop...
                std::mem::forget(msg.into_inner());

                Err(e.into())
            }
        }
    }

    pub async fn write_msg(&mut self, buf: &[u8]) -> io::Result<()> {
        let mut msg = self.new_message();

        let res: io::Result<()> = msg.write_all(buf).await;

        Self::dispose_msg(msg).await?;

        res
    }

    // Leaving here for later...
    // /// Use message within a callback and have it be closed automatically after
    // pub async fn with_msg<F, T, E>(&mut self, f: F) -> Result<T, E>
    // where
    //     F: for<'a> WithMsgCallback<'a, W, T, E>,
    //     E: From<io::Error>,
    // {
    //     let mut msg = self.new_message();
    //     let res: Result<T, E> = { f(&mut msg).await };
    //     match res {
    //         Ok(t) => match msg.flush().await {
    //             Ok(()) => match msg.into_inner().close().await {
    //                 Ok(()) => Ok(t),
    //                 Err(e) => Err(e.into()),
    //             },
    //             Err(e) => Err(io::Error::from(e).into()),
    //         },
    //         Err(e) => Err(e),
    //     }
    // }
}

#[cfg(feature = "encoding")]
impl<W: AsyncWrite + Unpin + 'static> AsyncFramedWriter<W> {
    /// serializes to a buffer then writes that out as an async message
    pub async fn write_buffered_object<T: serde::Serialize>(&mut self, value: &T) -> bincode::Result<()> {
        self.write_msg(&bincode::serialize(value)?).await?;

        Ok(())
    }
}

pub struct AsyncMessageWriter<'a, W> {
    w: &'a mut AsyncFramedWriter<W>,
    len: u64,

    header: [u8; 8],
    pos: usize,
}

impl<W: AsyncWrite + Unpin> AsyncMessageWriter<'_, W> {
    #[inline]
    fn inner(&mut self) -> Pin<&mut W> {
        Pin::new(&mut self.w.inner)
    }

    async fn try_close(&mut self) -> io::Result<()> {
        if self.len > 0 {
            // If the message didn't write as many bytes as it expected, then just fill with zeroes...
            io::copy(&mut io::repeat(0).take(self.len), self).await?;
        }

        // set length to 0 for closing frame
        self.header = [0; 8];

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
        panic!("AsyncMessageWriter cannot be dropped! Use `.close()` or `AsyncFramedWriter::dispose_msg(msg)` instead!");
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
            self.len = buf.len() as u64;
            self.pos = 0; // reset position so header can be written out
            self.header = self.len.to_be_bytes();
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
        Poll::Ready(match self.inner().poll_write(cx, &buf[..bytes_to_write]) {
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
    len: u64,

    header: [u8; 8],
    pos: usize,
}

impl<R: AsyncRead + Unpin> AsyncFramedReader<R> {
    pub fn new(inner: R) -> Self {
        AsyncFramedReader {
            inner,
            len: 0,
            header: [0; 8],
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
                    self.len = u64::from_be_bytes(self.header);

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

            self.len = u64::from_be_bytes(self.header);

            if self.len == 0 {
                return Poll::Ready(Ok(())); // EOF for end of message
            }
        }

        // automatically takes `min(remaining, len)`
        let mut b2 = buf.take(self.len as usize);

        match Pin::new(&mut self.inner).poll_read(cx, &mut b2) {
            Poll::Ready(Ok(())) => {
                let filled = b2.filled().len();

                // buf.take() doesn't actually update the original ReadBuf positions,
                // only writes to the *actual* underlying buffer, so update buf here or
                // else the parent reader will think it EOFed.
                unsafe {
                    buf.assume_init(filled);
                    buf.advance(filled);
                }

                self.len -= filled as u64;

                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

#[cfg(feature = "encoding")]
impl<R: AsyncRead + Unpin> AsyncFramedReader<R> {
    /// Read a bincode-encoded object message,
    /// after it has been buffered from the async stream.
    pub async fn read_buffered_object<T>(&mut self) -> bincode::Result<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        match self.next_msg().await? {
            Some(msg) => {
                // pre-allocate first frame
                let mut buf = Vec::with_capacity(msg.len as usize);
                msg.read_to_end(&mut buf).await?;
                bincode::deserialize(&buf).map(Some)
            }
            None => Ok(None),
        }
    }
}
