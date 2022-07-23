use ftl::*;

use smol_str::SmolStr;

use schema::{asset::AssetFlags, SnowflakeExt};
use sdk::{api::AssetQuery, models::Snowflake};

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

    let flags = match route.raw_query().map(|query| schema::asset::parse(query)) {
        Some(Ok(q)) => match q {
            AssetQuery::Flags { flags } => AssetFlags::from_bits_truncate(flags as i16),
            AssetQuery::HumanReadable {
                quality,
                animated,
                with_alpha,
                ext,
            } => {
                let mut flags = AssetFlags::empty().with_quality(quality).with_alpha(with_alpha);

                if animated {
                    flags |= AssetFlags::ANIMATED;
                }

                match ext {
                    Some(ext) => flags.with_ext(&ext),
                    None => flags
                        .union(AssetFlags::FORMATS)
                        .difference(AssetFlags::MAYBE_UNSUPPORTED_FORMATS),
                }
            }
        },
        _ => AssetFlags::all()
            .difference(AssetFlags::MAYBE_UNSUPPORTED_FORMATS)
            .with_quality(80),
    };

    log::debug!("REQUESTED ASSET FLAGS: {}={:?}", flags.bits(), flags);

    crate::backend::cdn::get_asset(route, kind, kind_id, asset_id, is_head, false, flags).await
}
