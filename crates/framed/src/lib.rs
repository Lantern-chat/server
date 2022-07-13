use std::io::{self, BufWriter, Read, Write};

pub struct FramedWriter<W: Write> {
    inner: W,
    msg: u64,
}

impl<W: Write> FramedWriter<W> {
    pub fn new(inner: W) -> Self {
        FramedWriter { inner, msg: 1 }
    }

    /// Constructs a new message writer that will
    /// take care of closing the messagee and flushing
    /// the buffer on drop
    pub fn new_message<'a>(&'a mut self) -> BufWriter<MessageWriter<'a, W>> {
        self.msg += 1;
        BufWriter::new(MessageWriter { w: self })
    }

    fn write_header(&mut self, len: u64) -> io::Result<()> {
        let mut header = [0u8; 8 + 8];
        header[0..8].copy_from_slice(&self.msg.to_be_bytes());
        header[8..16].copy_from_slice(&len.to_be_bytes());
        self.inner.write_all(&header)
    }
}

#[cfg(feature = "encoding")]
impl<W: Write> FramedWriter<W> {
    /// Writes a bincode-encoded object as a message
    pub fn write_object<T: serde::Serialize>(&mut self, value: &T) -> bincode::Result<()> {
        let mut msg = self.new_message();
        bincode::serialize_into(&mut msg, value)?;
        match msg.into_inner() {
            Ok(w) => w.close()?,
            Err(e) => return Err(io::Error::from(e).into()),
        }
        Ok(())
    }
}

pub struct MessageWriter<'a, W: Write> {
    w: &'a mut FramedWriter<W>,
}

impl<W: Write> MessageWriter<'_, W> {
    fn try_close(&mut self) -> io::Result<()> {
        self.w.write_header(0)?;
        self.w.inner.flush()
    }

    /// Manually close this message, to handle errors
    pub fn close(mut self) -> io::Result<()> {
        self.try_close()?;

        std::mem::forget(self);

        Ok(())
    }
}

// On drop, write a closing frame and flush
impl<W: Write> Drop for MessageWriter<'_, W> {
    fn drop(&mut self) {
        let _ = self.try_close();
    }
}

impl<W: Write> Write for MessageWriter<'_, W> {
    #[inline]
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.write_all(data)?;
        Ok(data.len())
    }

    fn write_all(&mut self, mut data: &[u8]) -> io::Result<()> {
        // protection against accidental close frames
        if data.is_empty() {
            return Ok(());
        }

        self.w.write_header(data.len() as u64)?;

        // do-while, as the above branch ensured there is data to write
        loop {
            match self.w.inner.write(data) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "failed to write framed data",
                    ))
                }
                Ok(n) => data = &data[n..],
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }

            if data.is_empty() {
                break;
            }
        }

        Ok(())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.w.inner.flush()
    }
}

pub struct FramedReader<R: Read> {
    inner: R,
    msg: u64,
    len: u64,
}

impl<R: Read> FramedReader<R> {
    pub fn new(inner: R) -> FramedReader<R> {
        FramedReader {
            inner,
            msg: 0,
            len: 0,
        }
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
                Ok((msg, len)) => Ok({
                    // sometimes readers don't consume the closing frame
                    // so if that happens just skip it and try again
                    if len == 0 {
                        continue; // goto loop start
                    }

                    self.msg = msg;
                    self.len = len;

                    Some(self)
                }),
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
                Err(e) => Err(e),
            };
        }
    }
}

fn read_header<R: Read>(mut r: R) -> io::Result<(u64, u64)> {
    let mut header = [0u8; 8 + 8];
    r.read_exact(&mut header)?;

    let mut msg = [0u8; 8];
    let mut len = [0u8; 8];

    msg.copy_from_slice(&header[0..8]);
    len.copy_from_slice(&header[8..16]);

    Ok((u64::from_be_bytes(msg), u64::from_be_bytes(len)))
}

impl<R: Read> Read for FramedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // if the current frame has been fully read
        if self.len == 0 {
            match read_header(&mut self.inner) {
                Ok((msg, len)) => {
                    if len == 0 {
                        return Ok(0); // EOF for end of message
                    }

                    self.msg = msg;
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
