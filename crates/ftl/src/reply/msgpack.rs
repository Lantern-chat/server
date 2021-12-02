use super::*;

use bytes::Bytes;
use futures::{Stream, StreamExt};

pub struct MsgPack {
    inner: Result<Bytes, ()>,
}

pub fn try_msgpack<T: serde::Serialize>(value: &T, named: bool) -> Result<MsgPack, rmp_serde::encode::Error> {
    let res = match named {
        true => rmp_serde::to_vec_named(value),
        false => rmp_serde::to_vec(value),
    };

    Ok(MsgPack {
        inner: match res {
            Ok(v) => Ok(Bytes::from(v)),
            Err(e) => return Err(e),
        },
    })
}

pub fn msgpack<T: serde::Serialize>(value: &T, named: bool) -> MsgPack {
    match try_msgpack(value, named) {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("MsgPack Reply error: {}", e);
            MsgPack { inner: Err(()) }
        }
    }
}

impl Reply for MsgPack {
    fn into_response(self) -> Response {
        match self.inner {
            Ok(body) => Body::from(body)
                .with_header(ContentType::from(mime::APPLICATION_MSGPACK))
                .into_response(),
            Err(()) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub struct MsgPackStream {
    body: Body,
}

pub fn array_stream<T, E>(
    stream: impl Stream<Item = Result<T, E>> + Send + 'static,
    named: bool,
) -> impl Reply
where
    T: serde::Serialize + Send + Sync + 'static,
    E: Into<Box<dyn std::error::Error + Send + Sync>> + Send + Sync + 'static,
{
    let (mut sender, body) = Body::channel();

    tokio::spawn(async move {
        futures::pin_mut!(stream);

        let mut buffer = Vec::with_capacity(128);

        let error: Result<(), Box<dyn std::error::Error + Send + Sync>> = loop {
            match stream.next().await {
                Some(Ok(ref value)) => {
                    let pos = buffer.len();

                    let res = match named {
                        true => rmp_serde::encode::write_named(&mut buffer, value),
                        false => rmp_serde::encode::write(&mut buffer, value),
                    };

                    if let Err(e) = res {
                        buffer.truncate(pos); // revert back to previous item
                        break Err(e.into());
                    }
                }
                Some(Err(e)) => break Err(e.into()),
                None => break Ok(()),
            }

            // Flush buffer at 8KiB
            if buffer.len() >= (1024 * 8) {
                let chunk = Bytes::from(std::mem::take(&mut buffer));
                if let Err(e) = sender.send_data(chunk).await {
                    log::error!("Error sending MessagePack chunk: {}", e);
                    return sender.abort();
                }
            }
        };

        if let Err(e) = sender.send_data(buffer.into()).await {
            log::error!("Error sending MessagePack chunk: {}", e);
            return sender.abort();
        }

        if let Err(e) = error {
            log::error!("Error serializing MessagePack stream: {}", e);
        }
    });

    MsgPackStream { body }
}

impl Reply for MsgPackStream {
    fn into_response(self) -> Response {
        self.body
            .with_header(ContentType::from(mime::APPLICATION_MSGPACK))
            .into_response()
    }
}
