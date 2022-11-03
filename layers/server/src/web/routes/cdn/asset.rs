use ftl::*;

use smol_str::SmolStr;

use sdk::{
    api::asset::{AssetFlags, AssetQuery},
    models::Snowflake,
};

use crate::backend::{cdn::AssetKind, util::encrypted_asset::decrypt_snowflake};

use super::ApiResponse;
use crate::{Error, ServerState};

pub enum PlainAssetKind {
    User,
    Room,
    Party,
    Role,
    Emote,
    Sticker,
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
            "emote" => PlainAssetKind::Emote,
            "sticker" => PlainAssetKind::Sticker,
            _ => return Err(Error::NotFound),
        },
    };

    let Some(Ok(kind_id)) = route.next().param::<Snowflake>() else { return Err(Error::BadRequest) };

    let mut asset_id = kind_id;

    let kind = match plain_kind {
        PlainAssetKind::Emote => AssetKind::Emote,
        PlainAssetKind::Sticker => AssetKind::Sticker,
        _ => {
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
            asset_id = match route.segment() {
                Exact(avatar) => match decrypt_snowflake(&route.state, avatar) {
                    Some(id) => id,
                    None => return Err(Error::BadRequest),
                },
                _ => return Err(Error::BadRequest),
            };

            kind
        }
    };

    let is_head = route.method() == Method::HEAD;

    let flags = match route.raw_query().map(|query| schema::asset::parse(query)) {
        Some(Ok(q)) => q.into(),
        _ => AssetFlags::all()
            .difference(AssetFlags::MAYBE_UNSUPPORTED_FORMATS)
            .with_quality(80),
    };

    log::debug!("REQUESTED ASSET FLAGS: {}={:?}", flags.bits(), flags);

    crate::backend::cdn::get_asset(route, kind, kind_id, asset_id, is_head, false, flags).await
}
