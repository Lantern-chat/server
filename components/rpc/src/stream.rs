use std::marker::PhantomData;

use crate::error::ApiError;
use futures::future::Either;
use futures::{Stream, StreamExt};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use std::io::{self, ErrorKind};

use framed::tokio::{AsyncFramedReader, AsyncFramedWriter};

use rkyv::ser::serializers::AllocSerializer;
use rkyv::{ser::Serializer, Serialize};

pub async fn encode_item<T, E, W, const N: usize>(
    out: AsyncFramedWriter<W>,
    item: Result<T, E>,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize<AllocSerializer<N>>,
    ApiError: From<E>,
{
    // stream::iter is more efficient
    encode_stream(out, Ok(futures::stream::iter([item]))).await
}

pub async fn encode_stream<T, E, W, const N: usize>(
    mut out: AsyncFramedWriter<W>,
    stream: Result<impl Stream<Item = Result<T, E>>, E>,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize<AllocSerializer<N>>,
    ApiError: From<E>,
{
    let mut serializer = AllocSerializer::default();

    let mut stream = std::pin::pin!(match stream {
        Ok(stream) => Either::Left(stream),
        Err(err) => Either::Right(futures::stream::iter([Err(err)])),
    });

    while let Some(item) = stream.next().await {
        let item: Result<T, ApiError> = match item {
            Ok(item) => Ok(item),
            Err(e) => Err(ApiError::from(e)),
        };

        if let Err(e) = serializer.serialize_value(&item) {
            log::error!("Error serializing streamed item: {e}");
            serializer.reset();
            continue;
        }

        let mut msg = out.new_message();
        msg.write_all(serializer.serializer().inner().as_slice()).await?;
        serializer.reset(); // immediately free buffers before flushing
        AsyncFramedWriter::dispose_msg(msg).await?;

        if item.is_err() {
            break; // only send one trailing error for logging
        }
    }

    Ok(())
}

pub struct RecvStream<T, R: AsyncRead + Unpin> {
    stream: AsyncFramedReader<R>,
    buffer: rkyv::AlignedVec,
    _ty: PhantomData<T>,
}

impl<T, R: AsyncRead + Unpin> RecvStream<T, R> {
    pub fn new(stream: R) -> Self {
        RecvStream {
            stream: AsyncFramedReader::new(stream),
            buffer: rkyv::AlignedVec::new(),
            _ty: PhantomData,
        }
    }
}

impl<T, R: AsyncRead + Unpin> RecvStream<T, R>
where
    T: rkyv::Archive,
    rkyv::Archived<T>: for<'a> rkyv::CheckBytes<rkyv::validation::validators::DefaultValidator<'a>>,
{
    pub async fn recv(&mut self) -> Result<Option<&rkyv::Archived<Result<T, ApiError>>>, io::Error> {
        let Some(msg) = self.stream.next_msg().await? else {
            return Ok(None);
        };

        self.buffer.resize(msg.len() as usize, 0);
        msg.read_exact(&mut self.buffer[..]).await?;

        let msg = match rkyv::check_archived_root::<Result<T, ApiError>>(&self.buffer) {
            Ok(msg) => msg,
            Err(e) => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("Error reading archived value: {e}"),
                ));
            }
        };

        Ok(Some(msg))
    }
}
