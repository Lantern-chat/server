use ftl::*;

use smol_str::SmolStr;

use sdk::models::Snowflake;
use schema::SnowflakeExt;

use crate::ctrl::util::encrypted_asset::decrypt_snowflake;
use crate::ctrl::Error;
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

    let file_id = match route.next().param::<SmolStr>() {
        Some(Ok(avatar)) => match decrypt_snowflake(&route.state, &avatar) {
            Some(id) => id,
            None => return StatusCode::BAD_REQUEST.into_response(),
        },
        _ => return StatusCode::BAD_REQUEST.into_response(),
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
        false,
    )
    .await
    {
        Err(e) => ApiError::err(e).into_response(),
        Ok(res) => res,
    }
}
