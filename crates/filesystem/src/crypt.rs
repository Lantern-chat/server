use aes::cipher::{StreamCipher, StreamCipherSeek};

use crate::store::Aes256Ctr;

use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, BufWriter, ReadBuf};

pin_project_lite::pin_project! {
    pub struct EncryptedFile<F> {
        #[pin]
        inner: F,

        cipher: Aes256Ctr,

        write_buf: Vec<u8>,
    }
}

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

impl<F> EncryptedFile<F> {
    pub fn new(inner: F, cipher: Aes256Ctr) -> Self {
        EncryptedFile {
            inner,
            cipher,
            write_buf: Vec::new(),
        }
    }

    pub fn new_write(inner: F, cipher: Aes256Ctr) -> EncryptedFile<BufWriter<F>>
    where
        F: AsyncWrite,
    {
        // buffer with 256KiB to avoid rewinding the cipher as often
        EncryptedFile::new(BufWriter::with_capacity(1024 * 256, inner), cipher)
    }

    pub fn new_write_sync(inner: F, cipher: Aes256Ctr) -> EncryptedFile<io::BufWriter<F>>
    where
        F: io::Write,
    {
        EncryptedFile::new(io::BufWriter::with_capacity(1024 * 256, inner), cipher)
    }

    pub fn get_ref(&self) -> &F {
        &self.inner
    }
}

impl<F: io::Read> io::Read for EncryptedFile<F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.inner.read(buf)?;

        self.cipher.apply_keystream(&mut buf[..len]);

        Ok(len)
    }
}

impl<F: AsyncRead + Unpin> AsyncRead for EncryptedFile<F> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let prev_filled_length = buf.filled().len();

        let this = self.project();

        match this.inner.poll_read(cx, buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => {
                this.cipher.apply_keystream(
                    buf.filled_mut()
                        .get_mut(prev_filled_length..)
                        .expect("Error getting newly filled buffer"),
                );

                Poll::Ready(Ok(()))
            }
        }
    }
}

impl<F: io::Write> io::Write for EncryptedFile<F> {
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let pos: u64 = self.cipher.current_pos();

        self.write_buf.clear();
        self.write_buf.extend_from_slice(&buf);

        self.cipher.apply_keystream(self.write_buf.as_mut_slice());

        let bytes = self.inner.write(self.write_buf.as_slice())?;

        if bytes < self.write_buf.len() {
            // partial rewind
            self.cipher.seek(pos + bytes as u64);
        }

        Ok(bytes)
    }
}

impl<F: AsyncWrite + Unpin> AsyncWrite for EncryptedFile<F> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.project();

        this.write_buf.clear();
        this.write_buf.extend_from_slice(&buf);

        let pos: u64 = this.cipher.current_pos();
        this.cipher.apply_keystream(this.write_buf.as_mut_slice());

        match this.inner.poll_write(cx, this.write_buf.as_slice()) {
            Poll::Pending => {
                // rewind cipher...
                this.cipher.seek(pos);
                Poll::Pending
            }
            Poll::Ready(Ok(bytes)) => {
                if bytes < this.write_buf.len() {
                    // partial rewind...
                    this.cipher.seek(pos + bytes as u64);
                }

                Poll::Ready(Ok(bytes))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().inner.poll_shutdown(cx)
    }
}

impl<F: io::Seek> io::Seek for EncryptedFile<F> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let pos = self.inner.seek(pos)?;
        self.cipher.seek(pos);
        Ok(pos)
    }
}

impl<F: AsyncSeek + Unpin> AsyncSeek for EncryptedFile<F> {
    fn start_seek(self: Pin<&mut Self>, position: io::SeekFrom) -> io::Result<()> {
        self.project().inner.start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
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
