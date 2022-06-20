use ftl::*;

use sdk::models::Snowflake;

use super::ApiResponse;
use crate::{backend::api::cdn::FileKind, Error, ServerState};

pub async fn attachments(mut route: Route<ServerState>) -> ApiResponse {
    let room_id = match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => room_id,
        _ => return Err(Error::BadRequest),
    };

    let attachment_id = match route.next().param::<Snowflake>() {
        Some(Ok(attachment_id)) => attachment_id,
        _ => return Err(Error::BadRequest),
    };

    let filename = match route.next().segment() {
        Exact(filename) => urlencoding::decode(filename)?.into(),
        _ => return Err(Error::BadRequest),
    };

    let is_head = route.method() == Method::HEAD;

    let download = route.raw_query() == Some("download");

    crate::backend::api::cdn::get_file(
        route,
        room_id,
        attachment_id,
        FileKind::Attachment,
        Some(filename),
        is_head,
        download,
    )
    .await
}
