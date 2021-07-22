use aes::{
    cipher::{StreamCipher, StreamCipherSeek},
    Aes256Ctr,
};

use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, BufWriter, ReadBuf};

#[pin_project::pin_project]
pub struct EncryptedFile<F> {
    #[pin]
    inner: F,

    cipher: Aes256Ctr,
}

use std::io::{self, Result as IoResult, SeekFrom};
use std::pin::Pin;
use std::task::{Context, Poll};

impl<F> EncryptedFile<F> {
    pub fn new_read(inner: F, cipher: Aes256Ctr) -> Self {
        EncryptedFile { inner, cipher }
    }

    pub fn new_write(inner: F, cipher: Aes256Ctr) -> EncryptedFile<BufWriter<F>>
    where
        F: AsyncWrite,
    {
        EncryptedFile::new(BufWriter::new(inner), cipher)
    }
}

impl<F: AsyncRead + Unpin> AsyncRead for EncryptedFile<F> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<IoResult<()>> {
        let prev_filled_length = buf.filled().len();

        let this = self.project();

        match this.inner.poll_read(cx, buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => {
                this.cipher
                    .apply_keystream(&mut buf.filled_mut()[prev_filled_length..]);

                Poll::Ready(Ok(()))
            }
        }
    }
}

impl<F: AsyncWrite + Unpin> AsyncWrite for EncryptedFile<F> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        let this = self.project();

        let pos: u64 = this.cipher.current_pos();

        let mut buf = buf.to_vec();
        this.cipher.apply_keystream(&mut buf);

        match this.inner.poll_write(cx, &buf) {
            Poll::Pending => {
                // rewind cipher...
                this.cipher.seek(pos);
                Poll::Pending
            }
            Poll::Ready(res) => Poll::Ready(res),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        self.project().inner.poll_shutdown(cx)
    }
}

impl<F: AsyncSeek + Unpin> AsyncSeek for EncryptedFile<F> {
    fn start_seek(self: Pin<&mut Self>, position: io::SeekFrom) -> IoResult<()> {
        self.project().inner.start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<u64>> {
        let this = self.project();

        match this.inner.poll_complete(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(pos)) => {
                this.cipher.seek(pos);

                Poll::Ready(Ok(pos))
            }
        }
    }
}
