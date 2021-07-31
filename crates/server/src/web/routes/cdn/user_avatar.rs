use ftl::*;

use models::Snowflake;

use crate::{web::routes::api::ApiError, ServerState};

pub async fn user_avatar(mut route: Route<ServerState>) -> Response {
    let user_id = match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => room_id,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let party_id = match route.next().param::<Snowflake>() {
        Some(Ok(party_id)) => Some(party_id),
        None => None,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let is_head = route.method() == Method::HEAD;

    match crate::ctrl::cdn::get_file(
        route,
        user_id,
        user_id,
        party_id,
        crate::ctrl::cdn::FileKind::UserAvatar,
        None,
        is_head,
    )
    .await
    {
        Err(e) => ApiError::err(e).into_response(),
        Ok(res) => res,
    }
}
