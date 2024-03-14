use sdk::api::error::ApiError;

use futures_util::future::Either;
use futures_util::{Stream, StreamExt};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use std::io::{self, ErrorKind};

use framed::tokio::{AsyncFramedReader, AsyncFramedWriter};

use rkyv::ser::serializers::AllocSerializer;
use rkyv::{ser::Serializer, Serialize};

pub async fn encode_item<T, E, W, const N: usize>(out: W, item: Result<T, E>) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize<AllocSerializer<N>>,
    E: std::error::Error,
    ApiError: From<E>,
{
    // stream::iter is more efficient
    encode_stream(out, Ok(futures_util::stream::iter([item]))).await
}

pub async fn encode_stream<T, E, W, const N: usize>(
    out: W,
    stream: Result<impl Stream<Item = Result<T, E>>, E>,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize<AllocSerializer<N>>,
    E: std::error::Error,
    ApiError: From<E>,
{
    let mut out = AsyncFramedWriter::new(out);
    let mut serializer = AllocSerializer::default();

    let mut stream = std::pin::pin!(match stream {
        Ok(stream) => Either::Left(stream),
        Err(err) => Either::Right(futures_util::stream::iter([Err(err)])),
    });

    while let Some(item) = stream.next().await {
        let item = item.map_err(|err| {
            log::error!("Error in RPC encode stream: {err}");

            ApiError::from(err)
        });

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

pub struct RpcRecvReader<R: AsyncRead + Unpin> {
    stream: AsyncFramedReader<R>,
    buffer: rkyv::AlignedVec,
}

impl<R: AsyncRead + Unpin> RpcRecvReader<R> {
    pub fn new(stream: R) -> Self {
        RpcRecvReader {
            stream: AsyncFramedReader::new(stream),
            buffer: rkyv::AlignedVec::new(),
        }
    }
}

impl<R: AsyncRead + Unpin> RpcRecvReader<R> {
    pub async fn recv<T>(&mut self) -> Result<Option<&rkyv::Archived<T>>, io::Error>
    where
        T: rkyv::Archive,
        rkyv::Archived<T>: for<'a> rkyv::CheckBytes<rkyv::validation::validators::DefaultValidator<'a>>,
    {
        let Some(msg) = self.stream.next_msg().await? else {
            return Ok(None);
        };

        self.buffer.resize(msg.len() as usize, 0);
        msg.read_exact(&mut self.buffer[..]).await?;

        match rkyv::check_archived_root::<T>(&self.buffer) {
            Ok(msg) => Ok(Some(msg)),
            Err(e) => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Error reading archived value: {e}"),
            )),
        }
    }
}
