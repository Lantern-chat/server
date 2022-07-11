use std::io::{self, BufWriter, Read, Write};

const MSG_BODY: u32 = 1;
const MSG_CLOSE: u32 = 2;

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

    fn write_header(&mut self, len: u32, flag: u32) -> io::Result<()> {
        let mut header = [0u8; 8 + 4 + 4];

        header[0..8].copy_from_slice(&self.msg.to_be_bytes());
        header[8..12].copy_from_slice(&len.to_be_bytes());
        header[12..16].copy_from_slice(&flag.to_be_bytes());

        self.inner.write_all(&header)
    }
}

pub struct MessageWriter<'a, W: Write> {
    w: &'a mut FramedWriter<W>,
}

impl<W: Write> MessageWriter<'_, W> {
    fn try_close(&mut self) -> io::Result<()> {
        self.w.write_header(0, MSG_CLOSE)?;
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
        self.w.write_header(data.len() as u32, MSG_BODY)?;

        while !data.is_empty() {
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
    len: u32,
}

impl<R: Read> FramedReader<R> {
    pub fn new(inner: R) -> FramedReader<R> {
        FramedReader {
            inner,
            msg: 0,
            len: 0,
        }
    }

    /// Once the previous message is finished,
    /// this will try to begin reading the next message.
    pub fn next_msg<'a>(&'a mut self) -> io::Result<Option<&'a mut Self>> {
        if self.len > 0 {
            return Ok(Some(self));
        }

        match read_header(&mut self.inner) {
            Ok((msg, len, flags)) => Ok({
                // sometimes readers don't consume the closing frame
                // so if that happens just skip it and recurse.
                if flags == MSG_CLOSE {
                    return self.next_msg();
                }

                self.msg = msg;
                self.len = len;

                Some(self)
            }),
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e),
        }
    }
}

fn read_header<R: Read>(mut r: R) -> io::Result<(u64, u32, u32)> {
    let mut header = [0u8; 8 + 4 + 4];
    r.read_exact(&mut header)?;

    let mut msg = [0u8; 8];
    let mut len = [0u8; 4];
    let mut flags = [0u8; 4];

    msg.copy_from_slice(&header[0..8]);
    len.copy_from_slice(&header[8..12]);
    flags.copy_from_slice(&header[12..16]);

    Ok((
        u64::from_be_bytes(msg),
        u32::from_be_bytes(len),
        u32::from_be_bytes(flags),
    ))
}

impl<R: Read> Read for FramedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // if the current frame has been fully read
        if self.len == 0 {
            match read_header(&mut self.inner) {
                Ok((msg, len, flags)) => {
                    if flags == MSG_CLOSE {
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
        self.len -= bytes_read as u32;

        Ok(bytes_read)
    }
}
