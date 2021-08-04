use ftl::*;

use models::Snowflake;
use schema::SnowflakeExt;

use crate::{util::hex::HexidecimalInt, web::routes::api::ApiError, ServerState};

pub async fn user_avatar(mut route: Route<ServerState>) -> Response {
    let user_id = match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => room_id,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let avatar = match route.next().param::<HexidecimalInt<u128>>() {
        Some(Ok(avatar)) => avatar,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    // decrypt file_id
    let file_id = match Snowflake::decrypt(avatar.0, route.state.config.sf_key) {
        Some(id) => id,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    //log::trace!("User avatar file id: {}", file_id);

    let is_head = route.method() == Method::HEAD;

    match crate::ctrl::cdn::get_file(
        route,
        user_id,
        file_id,
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
