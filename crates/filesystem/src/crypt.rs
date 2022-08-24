use aes::cipher::{StreamCipher, StreamCipherSeek};

use crate::store::Aes256Ctr;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncWrite, AsyncWriteExt, ReadBuf};

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

const BUFFER_SIZE: usize = 1024 * 256;

impl<F> EncryptedFile<F>
where
    F: AsyncWrite + Unpin,
{
    async fn do_write(&mut self) -> Result<usize, io::Error> {
        let len = self.write_buf.len();

        if len > 0 {
            self.cipher.apply_keystream(&mut self.write_buf);

            self.inner.write_all(&self.write_buf).await?;

            self.write_buf.clear();
        }

        Ok(len)
    }

    pub async fn write_buf(&mut self, src: &[u8]) -> Result<(), io::Error> {
        if (self.write_buf.len() + src.len()) > (1024 * 256) {
            self.do_write().await?;
        }

        // NOTE: `src` is typically already allocated before calling this,
        // so at most this just doubles the allocated memory used.
        self.write_buf.extend_from_slice(src);

        Ok(())
    }

    pub async fn copy_from<R>(&mut self, mut src: R) -> Result<usize, io::Error>
    where
        R: AsyncRead + Unpin,
    {
        let mut bytes_copied = 0;

        self.write_buf.reserve(BUFFER_SIZE);

        loop {
            let bytes_read = src.read_buf(&mut self.write_buf).await?;

            if 0 == bytes_read {
                break;
            }

            bytes_copied += bytes_read;

            if self.write_buf.len() >= BUFFER_SIZE {
                self.do_write().await?;
            }
        }

        self.flush().await?;

        Ok(bytes_copied)
    }

    pub async fn flush(&mut self) -> Result<(), io::Error> {
        self.do_write().await?;

        self.inner.flush().await
    }
}

// impl<F: io::Write> io::Write for EncryptedFile<F> {
//     fn flush(&mut self) -> io::Result<()> {
//         self.inner.flush()
//     }

//     fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
//         let pos: u64 = self.cipher.current_pos();

//         self.write_buf.clear();
//         self.write_buf.extend_from_slice(&buf);

//         self.cipher.apply_keystream(self.write_buf.as_mut_slice());

//         let bytes = self.inner.write(self.write_buf.as_slice())?;

//         if bytes < self.write_buf.len() {
//             // partial rewind
//             self.cipher.seek(pos + bytes as u64);
//         }

//         Ok(bytes)
//     }
// }

// impl<F: AsyncWrite + Unpin> AsyncWrite for EncryptedFile<F> {
//     fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
//         let this = self.project();

//         this.write_buf.clear();
//         this.write_buf.extend_from_slice(&buf);

//         println!("Encrypting chunk of size: {}", buf.len());

//         let pos: u64 = this.cipher.current_pos();
//         this.cipher.apply_keystream(this.write_buf.as_mut_slice());

//         match this.inner.poll_write(cx, this.write_buf.as_slice()) {
//             Poll::Pending => {
//                 // rewind cipher...
//                 this.cipher.seek(pos);
//                 Poll::Pending
//             }
//             Poll::Ready(Ok(bytes)) => {
//                 if bytes < this.write_buf.len() {
//                     // partial rewind...
//                     this.cipher.seek(pos + bytes as u64);
//                 }

//                 Poll::Ready(Ok(bytes))
//             }
//             Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
//         }
//     }

//     fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
//         self.project().inner.poll_flush(cx)
//     }

//     fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
//         self.project().inner.poll_shutdown(cx)
//     }
// }

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
