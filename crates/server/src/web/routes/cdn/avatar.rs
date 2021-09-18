use ftl::*;

use models::Snowflake;
use schema::SnowflakeExt;
use util::hex::HexidecimalInt;

use crate::{web::routes::api::ApiError, ServerState};

use crate::ctrl::cdn::FileKind;

pub enum AvatarKind {
    User,
    Room,
    Party,
    Role,
}

pub async fn avatar(mut route: Route<ServerState>, kind: AvatarKind) -> Response {
    let kind_id = match route.next().param::<Snowflake>() {
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

    let is_head = route.method() == Method::HEAD;

    match crate::ctrl::cdn::get_file(
        route,
        kind_id,
        file_id,
        match kind {
            AvatarKind::Party => FileKind::PartyAvatar,
            AvatarKind::Room => FileKind::RoomAvatar,
            AvatarKind::Role => FileKind::RoleAvatar,
            AvatarKind::User => FileKind::UserAvatar,
        },
        None,
        is_head,
    )
    .await
    {
        Err(e) => ApiError::err(e).into_response(),
        Ok(res) => res,
    }
}
