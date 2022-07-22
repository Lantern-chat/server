use ftl::*;

use smol_str::SmolStr;

use schema::SnowflakeExt;
use sdk::models::Snowflake;

use crate::backend::{cdn::AssetKind, util::encrypted_asset::decrypt_snowflake};

use super::ApiResponse;
use crate::{Error, ServerState};

pub enum PlainAssetKind {
    User,
    Room,
    Party,
    Role,
}

pub enum AssetSubKind {
    Avatar,
    Banner,
}

// cdn.lanternchat.net/user/user_id/banner/banner_id

pub async fn asset(mut route: Route<ServerState>) -> ApiResponse {
    let plain_kind = match route.segment() {
        End => unreachable!(),
        Exact(segment) => match segment {
            "user" => PlainAssetKind::User,
            "room" => PlainAssetKind::Room,
            "party" => PlainAssetKind::Party,
            "role" => PlainAssetKind::Role,
            _ => return Err(Error::NotFound),
        },
    };

    let kind_id = match route.next().param::<Snowflake>() {
        Some(Ok(id)) => id,
        _ => return Err(Error::BadRequest),
    };

    let sub_kind = match route.next().segment() {
        Exact("avatar") => AssetSubKind::Avatar,
        Exact("banner") => AssetSubKind::Banner,
        _ => return Err(Error::NotFound),
    };

    let kind = match (plain_kind, sub_kind) {
        (PlainAssetKind::Role, AssetSubKind::Avatar) => AssetKind::RoleAvatar,
        (PlainAssetKind::Party, AssetSubKind::Avatar) => AssetKind::PartyAvatar,
        (PlainAssetKind::Party, AssetSubKind::Banner) => AssetKind::PartyBanner,
        (PlainAssetKind::User, AssetSubKind::Avatar) => AssetKind::UserAvatar,
        (PlainAssetKind::User, AssetSubKind::Banner) => AssetKind::UserBanner,
        _ => return Err(Error::NotFound),
    };

    route.next();
    let asset_id = match route.segment() {
        Exact(avatar) => match decrypt_snowflake(&route.state, avatar) {
            Some(id) => id,
            None => return Err(Error::BadRequest),
        },
        _ => return Err(Error::BadRequest),
    };

    let is_head = route.method() == Method::HEAD;

    crate::backend::cdn::get_asset(route, kind, kind_id, asset_id, is_head, false).await
}
