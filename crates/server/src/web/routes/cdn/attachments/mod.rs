use ftl::*;

use models::Snowflake;

use crate::{web::routes::api::ApiError, ServerState};

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

    match crate::ctrl::cdn::get_file(
        route,
        room_id,
        attachment_id,
        crate::ctrl::cdn::FileKind::Attachment,
        Some(filename),
        is_head,
    )
    .await
    {
        Err(e) => ApiError::err(e).into_response(),
        Ok(res) => res,
    }
}
