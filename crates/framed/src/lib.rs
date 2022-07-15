#[cfg(feature = "tokio")]
pub mod tokio;

use std::io::{self, BufWriter, Read, Write};

pub struct FramedWriter<W: Write> {
    inner: W,
}

impl<W: Write> FramedWriter<W> {
    pub fn new(inner: W) -> Self {
        FramedWriter { inner }
    }

    /// Constructs a new message writer that will
    /// take care of closing the messagee and flushing
    /// the buffer on drop
    pub fn new_message<'a>(&'a mut self) -> BufWriter<MessageWriter<'a, W>> {
        BufWriter::new(MessageWriter { w: self, len: 0 })
    }

    fn write_header(&mut self, len: u64) -> io::Result<()> {
        self.inner.write_all(&len.to_be_bytes())
    }

    /// Use message within a callback and have it be closed automatically after
    pub fn with_msg<F, T, E>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut BufWriter<MessageWriter<W>>) -> Result<T, E>,
        E: From<io::Error>,
    {
        let mut msg = self.new_message();

        f(&mut msg).and_then(|t| match msg.into_inner() {
            Ok(w) => match w.close() {
                Ok(()) => Ok(t),
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(io::Error::from(e).into()),
        })
    }
}

#[cfg(feature = "encoding")]
impl<W: Write> FramedWriter<W> {
    /// Writes a bincode-encoded object as a message
    pub fn write_object<T: serde::Serialize>(&mut self, value: &T) -> bincode::Result<()> {
        self.with_msg(|msg| bincode::serialize_into(msg, value))
    }
}

pub struct MessageWriter<'a, W: Write> {
    w: &'a mut FramedWriter<W>,
    len: u64,
}

impl<W: Write> MessageWriter<'_, W> {
    fn try_close(&mut self) -> io::Result<()> {
        if self.len > 0 {
            // If the message didn't write as many bytes as it expected, then just fill with zeroes...
            io::copy(&mut io::repeat(0).take(self.len), self)?;
        }

        self.w.write_header(0)?;
        self.w.inner.flush()
    }

    /// Manually close this message, to handle errors
    pub fn close(mut self) -> io::Result<()> {
        let res = self.try_close();
        std::mem::forget(self);
        res
    }
}

// On drop, write a closing frame and flush
impl<W: Write> Drop for MessageWriter<'_, W> {
    fn drop(&mut self) {
        let _ = self.try_close();
    }
}

impl<W: Write> Write for MessageWriter<'_, W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.len == 0 && !data.is_empty() {
            self.len = data.len() as u64;
            self.w.write_header(self.len)?;
        }

        let bytes_to_write = data.len().min(self.len as usize);
        let bytes_written = self.w.inner.write(&data[..bytes_to_write])?;

        self.len -= bytes_written as u64;

        Ok(bytes_written)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.w.inner.flush()
    }
}

pub struct FramedReader<R: Read> {
    inner: R,
    len: u64,
}

impl<R: Read> FramedReader<R> {
    pub fn new(inner: R) -> FramedReader<R> {
        FramedReader { inner, len: 0 }
    }

    /// Throw away rest of the message
    fn sink(&mut self) -> io::Result<u64> {
        io::copy(self, &mut io::sink())
    }

    /// Once the previous message is finished,
    /// this will try to begin reading the next message.
    pub fn next_msg<'a>(&'a mut self) -> io::Result<Option<&'a mut Self>> {
        if self.len > 0 {
            // consume the rest of this message, INCLUDING THE CLOSING FRAME
            self.sink()?;
        }

        loop {
            return match read_header(&mut self.inner) {
                Ok(len) => Ok({
                    // sometimes readers don't consume the closing frame
                    // so if that happens just skip it and try again
                    if len == 0 {
                        continue; // goto loop start
                    }

                    self.len = len;

                    Some(self)
                }),
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
                Err(e) => Err(e),
            };
        }
    }
}

fn read_header<R: Read>(mut r: R) -> io::Result<u64> {
    let mut len = [0u8; 8];
    r.read_exact(&mut len)?;
    Ok(u64::from_be_bytes(len))
}

impl<R: Read> Read for FramedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // if the current frame has been fully read
        if self.len == 0 {
            match read_header(&mut self.inner) {
                Ok(len) => {
                    if len == 0 {
                        return Ok(0); // EOF for end of message
                    }

                    self.len = len;
                }
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    return Ok(0);
                }
                Err(e) => return Err(e),
            }
        }

        // read enough to fill the buffer or up to the end of the frame
        let can_be_filled = buf.len().min(self.len as usize);
        let bytes_read = self.inner.read(&mut buf[..can_be_filled])?;

        // mark as read
        self.len -= bytes_read as u64;

        Ok(bytes_read)
    }
}

#[cfg(feature = "encoding")]
impl<R: Read> FramedReader<R> {
    /// Read a bincode-encoded object message
    pub fn read_object<T>(&mut self) -> bincode::Result<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        match self.next_msg()? {
            Some(msg) => bincode::deserialize_from(msg).map(Some),
            None => Ok(None),
        }
    }
}
