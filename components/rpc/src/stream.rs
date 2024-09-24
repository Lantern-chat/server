use sdk::api::error::ApiError;

use futures_util::future::Either;
use futures_util::{Stream, StreamExt};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

use std::io::{self, ErrorKind};

use framed::tokio::{AsyncFramedReader, AsyncFramedWriter};

use rkyv::{
    api::high::{HighDeserializer, HighValidator},
    bytecheck::CheckBytes,
    rancor::{Error as RancorError, Strategy},
    ser::{
        allocator::{Arena, ArenaHandle},
        sharing::Share,
        Serializer,
    },
    util::AlignedVec,
    Archive, Archived, Deserialize, Portable, Serialize,
};

pub type DefaultHighSerializer<'a> =
    Strategy<Serializer<&'a mut Vec<u8>, ArenaHandle<'a>, &'a mut Share>, RancorError>;

pub async fn encode_item<T, E, W>(out: W, item: Result<T, E>) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: for<'a> Serialize<DefaultHighSerializer<'a>>,
    E: core::fmt::Debug,
    ApiError: From<E>,
{
    // stream::iter is more efficient
    encode_stream(out, Ok(futures_util::stream::iter([item]))).await
}

pub async fn encode_stream<T, E, W>(
    out: W,
    stream: Result<impl Stream<Item = Result<T, E>>, E>,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: for<'a> Serialize<DefaultHighSerializer<'a>>,
    E: core::fmt::Debug,
    ApiError: From<E>,
{
    let mut out = AsyncFramedWriter::new(out);

    let mut arena = Arena::new();
    let mut buffer = Vec::new();
    let mut share = Share::new();

    let mut stream = std::pin::pin!(match stream {
        Ok(stream) => Either::Left(stream),
        Err(err) => Either::Right(futures_util::stream::iter([Err(err)])),
    });

    while let Some(item) = stream.next().await {
        // coalesce errors into a single API error
        let item = item.map_err(|err| {
            log::error!("Error in RPC encode stream: {err:?}");
            ApiError::from(err)
        });

        buffer.clear();
        share.clear();

        let mut ser = Serializer::new(&mut buffer, arena.acquire(), &mut share);

        if let Err(e) = rkyv::api::serialize_using(&item, &mut ser) {
            log::error!("Error serializing streamed item: {e}");
            continue;
        }

        out.write_msg(&buffer).await?;

        if item.is_err() {
            break; // only send one trailing error for logging
        }
    }

    Ok(())
}

pub struct RpcRecvReader<R: AsyncRead + Unpin> {
    stream: AsyncFramedReader<R>,
    buffer: AlignedVec,
}

impl<R: AsyncRead + Unpin> RpcRecvReader<R> {
    pub fn new(stream: R) -> Self {
        RpcRecvReader {
            stream: AsyncFramedReader::new(stream),
            buffer: AlignedVec::new(),
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> R {
        self.stream.into_inner()
    }
}

impl<R: AsyncRead + Unpin> RpcRecvReader<R> {
    /// Reads from the underlying stream and returns the next message.
    pub async fn recv<'a, T>(&'a mut self) -> Result<Option<&'a Archived<T>>, io::Error>
    where
        T: Archive + 'a,
        Archived<T>: Portable + for<'b> CheckBytes<HighValidator<'b, RancorError>>,
    {
        let Some(msg) = self.stream.next_msg().await? else {
            return Ok(None);
        };

        self.buffer.resize(msg.len() as usize, 0);
        msg.read_exact(&mut self.buffer[..]).await?;

        match rkyv::access::<Archived<T>, RancorError>(&self.buffer) {
            Ok(msg) => Ok(Some(msg)),
            Err(e) => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Error reading archived value: {e}"),
            )),
        }
    }

    /// Converts the reader into a stream of deserialized values.
    pub fn recv_stream_deserialized<T>(self) -> impl Stream<Item = Result<T, io::Error>>
    where
        T: Archive,
        Archived<T>: Portable + for<'b> CheckBytes<HighValidator<'b, RancorError>>,
        Archived<T>: Deserialize<T, HighDeserializer<RancorError>>,
    {
        futures_util::stream::try_unfold(self, move |mut reader| async move {
            Ok(match reader.recv::<T>().await? {
                Some(msg) => {
                    let Ok(msg) = rkyv::deserialize(msg) else {
                        return Err(io::Error::new(
                            ErrorKind::InvalidData,
                            "Error deserializing archived value",
                        ));
                    };

                    Some((msg, reader))
                }
                None => None,
            })
        })
    }
}
