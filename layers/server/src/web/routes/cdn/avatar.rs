use ftl::*;

use smol_str::SmolStr;

use schema::SnowflakeExt;
use sdk::models::Snowflake;

use crate::backend::api::cdn::FileKind;
use crate::backend::util::encrypted_asset::decrypt_snowflake;

use super::ApiResponse;
use crate::{Error, ServerState};

pub enum AvatarKind {
    User,
    Room,
    Party,
    Role,
}

pub async fn avatar(mut route: Route<ServerState>, kind: AvatarKind) -> ApiResponse {
    let kind_id = match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => room_id,
        _ => return Err(Error::BadRequest),
    };

    let file_id = match route.next().param::<SmolStr>() {
        Some(Ok(avatar)) => match decrypt_snowflake(&route.state, &avatar) {
            Some(id) => id,
            None => return Err(Error::BadRequest),
        },
        _ => return Err(Error::BadRequest),
    };

    let is_head = route.method() == Method::HEAD;

    crate::backend::api::cdn::get_file(
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
}
