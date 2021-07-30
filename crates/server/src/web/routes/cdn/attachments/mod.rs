use std::{io::SeekFrom, str::FromStr, time::Instant};

use bytes::{Bytes, BytesMut};
use ftl::{fs::bytes_range, *};

use headers::{
    AcceptRanges, ContentLength, ContentRange, ContentType, HeaderMap, HeaderMapExt, HeaderValue, Range,
};
use hyper::Body;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use models::Snowflake;
use schema::flags::FileFlags;

use crate::{
    ctrl::Error,
    filesystem::store::{CipherOptions, OpenMode},
    web::routes::api::ApiError,
    ServerState,
};

pub async fn attachments(mut route: Route<ServerState>) -> Response {
    let room_id = match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => room_id,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let attachment_id = match route.next().param::<Snowflake>() {
        Some(Ok(attachment_id)) => attachment_id,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let filename = match route.next().segment() {
        Exact(filename) => match urlencoding::decode(filename) {
            Ok(filename) => filename.into_owned(),
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let is_head = route.method() == Method::HEAD;

    match get_attachment(route, room_id, attachment_id, filename, is_head).await {
        Err(e) => ApiError::err(e).into_response(),
        Ok(res) => res,
    }
}

async fn get_attachment(
    route: Route<ServerState>,
    room_id: Snowflake,
    attachment_id: Snowflake,
    filename: String,
    is_head: bool,
) -> Result<Response, Error> {
    let range: Option<Range> = route.header();

    let route_start = route.start;
    let state = route.state;

    let db = state.db.read.get().await?;

    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .cols(&[Files::Size, Files::Flags, Files::Nonce, Files::Name, Files::Mime])
                    .from(
                        Files::inner_join(
                            Attachments::inner_join_table::<Messages>()
                                .on(Attachments::MessageId.equals(Messages::Id)),
                        )
                        .on(Files::Id.equals(Attachments::FileId)),
                    )
                    .and_where(Files::Id.equals(Var::of(Files::Id)))
                    .and_where(Messages::RoomId.equals(Var::of(Rooms::Id)))
            },
            &[&attachment_id, &room_id],
        )
        .await?;

    let row = match row {
        None => return Err(Error::NotFound),
        Some(row) => row,
    };

    let name: String = row.try_get(3)?;

    if name != filename {
        log::debug!("{:?} != {:?}", name, filename);
        return Err(Error::BadRequest);
    }

    let size: i32 = row.try_get(0)?;
    let flags = FileFlags::from_bits_truncate(row.try_get(1)?);
    let nonce: i64 = row.try_get(2)?;
    let mime: Option<String> = row.try_get(4)?;

    let options = CipherOptions {
        key: state.config.file_key,
        nonce: unsafe { std::mem::transmute([nonce, nonce]) },
    };

    let mut file = state
        .fs
        .open_crypt(attachment_id, OpenMode::Read, &options)
        .await?;

    let mut len = size as u64;

    let mut res = if is_head {
        Response::default()
    } else {
        let (start, end) = match bytes_range(range, len) {
            Err(_) => {
                return Ok(StatusCode::RANGE_NOT_SATISFIABLE
                    .with_header(ContentRange::unsatisfied_bytes(len))
                    .into_response())
            }
            Ok(range) => range,
        };

        let sub_len = end - start;

        let (mut sender, body) = Body::channel();

        let mut res = Response::new(body);

        if len != sub_len {
            *res.status_mut() = StatusCode::PARTIAL_CONTENT;

            res.headers_mut()
                .typed_insert(ContentRange::bytes(start..end, len).expect("valid ContentRange"));

            len = sub_len;
        }

        tokio::spawn(async move {
            if start != 0 {
                if let Err(e) = file.seek(SeekFrom::Start(start)).await {
                    log::error!("Error seeking file: {}", e);
                    return sender.abort();
                }
            }

            let mut buf = BytesMut::new();
            let mut len = sub_len;

            let buf_size = 1024 * 512; // 64Kb

            while len != 0 {
                if buf.capacity() - buf.len() < buf_size {
                    buf.reserve(buf_size);
                }

                let n = match file.read_buf(&mut buf).await {
                    Ok(n) => n as u64,
                    Err(err) => {
                        log::debug!("File read error: {}", err);
                        return sender.abort();
                    }
                };

                if n == 0 {
                    log::warn!("File read found EOF before expected length: {}", len);
                    break;
                }

                let mut chunk = buf.split().freeze();

                if n > len {
                    chunk = chunk.split_to(len as usize);
                    len = 0;
                } else {
                    len -= n;
                }

                if let Err(e) = sender.send_data(chunk).await {
                    log::error!("Error sending file chunk: {}", e);
                    return sender.abort();
                }
            }

            let elapsed = route_start.elapsed().as_secs_f64() * 1000.0;

            log::debug!("File transfer finished in {:.4}ms", elapsed);

            let mut trailers = HeaderMap::new();
            if let Ok(value) = HeaderValue::from_str(&format!("end;dur={:.4}", elapsed)) {
                trailers.insert("Server-Timing", value);

                if let Err(e) = sender.send_trailers(trailers).await {
                    log::warn!("Error sending trailers: {}", e);
                }
            } else {
                log::warn!("Unable to create trailer value");
            }

            drop(sender);
        });

        res
    };

    let headers = res.headers_mut();

    headers.typed_insert(ContentLength(len));
    headers.typed_insert(AcceptRanges::bytes());

    headers.insert(
        "Content-Disposition",
        HeaderValue::from_str(&format!("inline; filename=\"{}\"", name))?,
    );

    if let Some(mime) = mime {
        headers.insert("Content-Type", HeaderValue::from_str(&mime)?);
    } else {
        headers.typed_insert(ContentType::octet_stream());
    }

    Ok(res)
}
