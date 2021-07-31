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

pub enum FileKind {
    Attachment,
    UserAvatar,
}

pub async fn get_file(
    route: Route<ServerState>,
    kind_id: Snowflake,
    file_id: Snowflake,
    party_id: Option<Snowflake>,
    kind: FileKind,
    filename: Option<String>,
    is_head: bool,
) -> Result<Response, Error> {
    let range: Option<Range> = route.header();

    let route_start = route.start;
    let state = route.state;

    let db = state.db.read.get().await?;

    let row = match kind {
        FileKind::Attachment => {
            db.query_opt_cached_typed(|| select_attachment(), &[&file_id, &kind_id])
                .await?
        }
        FileKind::UserAvatar => match party_id {
            Some(party_id) => {
                db.query_opt_cached_typed(|| select_user_avatar(true), &[&kind_id, &party_id])
                    .await?
            }
            None => {
                db.query_opt_cached_typed(|| select_user_avatar(false), &[&kind_id])
                    .await?
            }
        },
    };

    let row = match row {
        None => return Err(Error::NotFound),
        Some(row) => row,
    };

    let name: String = row.try_get(4)?;

    if let Some(filename) = filename {
        if name != filename {
            log::debug!("{:?} != {:?}", name, filename);
            return Err(Error::BadRequest);
        }
    }

    // TODO: Determine what to do with flags?

    let file_id: Snowflake = row.try_get(0)?;
    let size: i32 = row.try_get(1)?;
    let flags = FileFlags::from_bits_truncate(row.try_get(2)?);
    let nonce: i64 = row.try_get(3)?;
    let mime: Option<String> = row.try_get(5)?;

    let options = CipherOptions {
        key: state.config.file_key,
        nonce: unsafe { std::mem::transmute([nonce, nonce]) },
    };

    let _fs_permit = state.fs_semaphore.acquire().await?;

    let mut file = state.fs.open_crypt(file_id, OpenMode::Read, &options).await?;

    let mut len = size as u64;

    // in debug mode, double-check length
    if cfg!(debug_assertions) {
        let real_len = file.get_len().await?;

        assert_eq!(len, real_len);
    }

    let mut res = if is_head {
        Response::default()
    } else {
        // parse byte range using ftl method
        let (start, end) = match bytes_range(range, len) {
            Err(_) => {
                return Ok(StatusCode::RANGE_NOT_SATISFIABLE
                    .with_header(ContentRange::unsatisfied_bytes(len))
                    .into_response())
            }
            Ok(range) => range,
        };

        // determine content length from range (if applicable)
        let sub_len = end - start;

        // setup body, sender and response
        let (mut sender, body) = Body::channel();
        let mut res = Response::new(body);

        // if the selected range is not the entire length, set applicable headers
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

            // TODO: Adjust the buffer size dynamically based on how long it took to transmite the previous buffer
            // so if the connection is fast use a larger buffer, and smaller buffers for slow connections.

            let mut buf = BytesMut::new();
            let mut len = sub_len;

            let buf_size = 1024 * 64; // 64Kb

            while len != 0 {
                if buf.capacity() - buf.len() < buf_size {
                    buf.reserve(buf_size);
                }

                let n = match file.read_buf(&mut buf).await {
                    Ok(n) => n as u64,
                    Err(err) => {
                        log::error!("File read error: {}", err);
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
                    log::warn!("Error sending file chunk: {}", e);
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

    // always try to display the file inline
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

use thorn::*;

fn select_attachment() -> impl AnyQuery {
    use schema::*;

    Query::select()
        .cols(&[
            /* 0 */ Files::Id,
            /* 1 */ Files::Size,
            /* 2 */ Files::Flags,
            /* 3 */ Files::Nonce,
            /* 4 */ Files::Name,
            /* 5 */ Files::Mime,
        ])
        .from(
            Files::inner_join(
                Attachments::inner_join_table::<Messages>().on(Attachments::MessageId.equals(Messages::Id)),
            )
            .on(Files::Id.equals(Attachments::FileId)),
        )
        .and_where(Files::Id.equals(Var::of(Files::Id)))
        .and_where(Messages::RoomId.equals(Var::of(Rooms::Id)))
        .limit_n(1)
}

fn select_user_avatar(with_party: bool) -> impl AnyQuery {
    use schema::*;

    let mut query = Query::select()
        .cols(&[
            Files::Id,
            Files::Size,
            Files::Flags,
            Files::Nonce,
            Files::Name,
            Files::Mime,
        ])
        .from(Files::inner_join_table::<UserAvatars>().on(Files::Id.equals(UserAvatars::FileId)))
        .and_where(UserAvatars::UserId.equals(Var::of(Users::Id)))
        .limit_n(1);

    if with_party {
        query = query.and_where(UserAvatars::PartyId.equals(Var::of(Party::Id)));
    } else {
        query = query.and_where(UserAvatars::PartyId.is_null());
    }

    query
}