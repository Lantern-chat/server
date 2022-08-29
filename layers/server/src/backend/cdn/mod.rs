use std::{io::SeekFrom, str::FromStr, time::Instant};

use bytes::{Bytes, BytesMut};
use ftl::{
    fs::{bytes_range, Cond, Conditionals},
    *,
};

use filesystem::store::{CipherOptions, OpenMode};
use futures::FutureExt;
use headers::{
    AcceptRanges, ContentLength, ContentRange, ContentType, HeaderMap, HeaderMapExt, HeaderValue,
    IfModifiedSince, LastModified, Range,
};
use hyper::Body;
use smol_str::SmolStr;
use thorn::pg::ToSql;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use schema::flags::FileFlags;
use sdk::models::{AssetFlags, Snowflake};

use crate::{Error, ServerState};

pub mod sendfile;

pub enum AssetKind {
    UserAvatar,
    UserBanner,
    RoleAvatar,
    PartyAvatar,
    PartyBanner,
    RoomAvatar,
    Emote,
    Sticker,
}

pub async fn get_asset(
    route: Route<ServerState>,
    kind: AssetKind,
    kind_id: Snowflake,
    asset_id: Snowflake,
    is_head: bool,
    download: bool,
    flags: AssetFlags,
) -> Result<Response, Error> {
    let range = route.header::<headers::Range>();
    let last_modified = LastModified::from(asset_id.system_timestamp());

    let range = match Conditionals::new(&route, range).check(Some(last_modified)) {
        Cond::NoBody(resp) => return Ok(resp),
        Cond::WithBody(range) => range,
    };

    let Route { start, state, .. } = route;

    let db = state.db.read.get().await?;

    use query::{AssetParams, Parameters};

    let params = AssetParams {
        asset_id,
        kind_id,
        flags: flags.bits(),
    };
    let params = &params.as_params();

    // // boxing these is probably cheaper than the compiled state machines of all of them combined
    #[rustfmt::skip]
    let row_future = match kind {
        AssetKind::UserAvatar   => db.query_opt_cached_typed(|| query::select_asset(AssetKind::UserAvatar), params).boxed(),
        AssetKind::UserBanner   => db.query_opt_cached_typed(|| query::select_asset(AssetKind::UserBanner), params).boxed(),
        AssetKind::RoleAvatar   => db.query_opt_cached_typed(|| query::select_asset(AssetKind::RoleAvatar), params).boxed(),
        AssetKind::RoomAvatar   => db.query_opt_cached_typed(|| query::select_asset(AssetKind::RoomAvatar), params).boxed(),
        AssetKind::PartyAvatar  => db.query_opt_cached_typed(|| query::select_asset(AssetKind::PartyAvatar), params).boxed(),
        AssetKind::PartyBanner  => db.query_opt_cached_typed(|| query::select_asset(AssetKind::PartyBanner), params).boxed(),
        AssetKind::Emote        => db.query_opt_cached_typed(|| query::select_asset(AssetKind::Emote), params).boxed(),
        AssetKind::Sticker      => db.query_opt_cached_typed(|| query::select_asset(AssetKind::Sticker), params).boxed(),
    };

    let row = match row_future.await {
        Ok(None) => return Err(Error::NotFound),
        Ok(Some(row)) => row,
        Err(e) => return Err(e.into()),
    };

    drop(db);

    let meta = query::parse_file(&row)?;

    sendfile::send_file(state, meta, is_head, download, range, last_modified, Some(start)).await
}

pub async fn get_attachment(
    route: Route<ServerState>,
    room_id: Snowflake,
    file_id: Snowflake,
    provided_name: Option<&str>,
    is_head: bool,
    download: bool,
) -> Result<Response, Error> {
    let last_modified = LastModified::from(file_id.system_timestamp());

    let range = route.header::<headers::Range>();
    let range = match Conditionals::new(&route, range).check(Some(last_modified)) {
        Cond::NoBody(resp) => return Ok(resp),
        Cond::WithBody(range) => range,
    };

    let Route { start, state, .. } = route;

    let db = state.db.read.get().await?;

    let row = db
        .query_opt_cached_typed(|| query::select_attachment(), &[&file_id, &room_id])
        .await?;

    let row = match row {
        Some(row) => row,
        None => return Err(Error::NotFound),
    };

    drop(db);

    let meta = query::parse_file(&row)?;

    if let Some(provided_name) = provided_name {
        if meta.name != provided_name {
            log::debug!("{:?} != {:?}", meta.name, provided_name);
            return Err(Error::NotFound);
        }
    }

    sendfile::send_file(state, meta, is_head, download, range, last_modified, Some(start)).await
}

use query::ParsedFile;

mod query {
    pub use schema::*;
    pub use thorn::*;

    use super::{AssetFlags, AssetKind};

    indexed_columns! {
        pub enum Columns {
            Files::Id,
            Files::Size,
            Files::Flags,
            Files::Nonce,
            Files::Name,
            Files::Mime,
        }
    }

    pub struct ParsedFile<'a> {
        pub id: Snowflake,
        pub size: i32,
        pub flags: flags::FileFlags,
        pub nonce: i64,
        pub name: &'a str,
        pub mime: Option<&'a str>,
    }

    pub fn parse_file<'a>(row: &'a db::Row) -> Result<ParsedFile<'a>, db::pg::Error> {
        Ok(ParsedFile {
            id: row.try_get(Columns::id())?,
            size: row.try_get(Columns::size())?,
            flags: flags::FileFlags::from_bits_truncate(row.try_get(Columns::flags())?),
            nonce: row.try_get(Columns::nonce())?,
            name: row.try_get(Columns::name())?,
            mime: row.try_get(Columns::mime())?,
        })
    }

    pub fn select_attachment() -> impl AnyQuery {
        Query::select()
            .cols(Columns::default())
            .from(
                Attachments::inner_join_table::<Messages>()
                    .on(Attachments::MessageId.equals(Messages::Id))
                    .inner_join_table::<Files>()
                    .on(Files::Id.equals(Attachments::FileId)),
            )
            .and_where(Files::Id.equals(Var::of(Files::Id)))
            .and_where(Messages::RoomId.equals(Var::of(Rooms::Id)))
    }

    params! {
        pub struct AssetParams {
            pub asset_id: Snowflake = UserAssetFiles::AssetId,
            pub kind_id: Snowflake = UserAssetFiles::AssetId,
            pub flags: i16 = UserAssetFiles::Flags,
        }
    }

    pub fn select_asset(kind: AssetKind) -> impl thorn::AnyQuery {
        let quality = UserAssetFiles::Flags.bit_and(AssetFlags::QUALITY.bits().lit());

        let mut q = Query::select()
            .cols(Columns::default())
            // select files of at least the given quality
            .and_where(quality.greater_than_equal(Builtin::least((
                AssetParams::flags().bit_and(AssetFlags::QUALITY.bits().lit()),
                100i16.lit(),
            ))))
            .and_where(
                UserAssetFiles::Flags
                    .has_any_bits(AssetParams::flags().bit_and(AssetFlags::FORMATS.bits().lit())),
            )
            .and_where(
                UserAssetFiles::Flags
                    .has_any_bits(AssetParams::flags().bit_and(AssetFlags::FLAGS.bits().lit()))
                    .or(UserAssetFiles::Flags.has_no_bits(AssetFlags::FLAGS.bits().lit())),
            )
            .order_by(
                // prioritize images with animation, then alpha, then without alpha
                // this is possible because the ANIMATED flag is higher than HAS_ALPHA
                UserAssetFiles::Flags
                    .bit_and(AssetFlags::FLAGS.bits().lit())
                    .descending(),
            )
            // order by file size, to pick the smallest one first
            .order_by(Files::Size.ascending())
            .limit_n(1);

        let from = UserAssetFiles::inner_join_table::<Files>().on(Files::Id.equals(UserAssetFiles::FileId));

        if !matches!(kind, AssetKind::Emote | AssetKind::Sticker) {
            q = q.and_where(UserAssetFiles::AssetId.equals(AssetParams::asset_id()));
        }

        #[rustfmt::skip]
        let q = match kind {
            AssetKind::UserAvatar => q
                .from(from.inner_join_table::<Profiles>().on(Profiles::AvatarId.equals(UserAssetFiles::AssetId)))
                .and_where(Profiles::UserId.equals(AssetParams::kind_id())),
            AssetKind::UserBanner => q
                .from(from.inner_join_table::<Profiles>().on(Profiles::BannerId.equals(UserAssetFiles::AssetId)))
                .and_where(Profiles::UserId.equals(AssetParams::kind_id())),
            AssetKind::RoleAvatar => q
                .from(from.inner_join_table::<Roles>().on(Roles::AvatarId.equals(UserAssetFiles::AssetId)))
                .and_where(Roles::Id.equals(AssetParams::kind_id())),
            AssetKind::RoomAvatar => q
                .from(from.inner_join_table::<Rooms>().on(Rooms::AvatarId.equals(UserAssetFiles::AssetId)))
                .and_where(Rooms::Id.equals(AssetParams::kind_id())),
            AssetKind::PartyAvatar => q
                .from(from.inner_join_table::<Party>().on(Party::AvatarId.equals(UserAssetFiles::AssetId)))
                .and_where(Party::Id.equals(AssetParams::kind_id())),
            AssetKind::PartyBanner => q
                .from(from.inner_join_table::<Party>().on(Party::BannerId.equals(UserAssetFiles::AssetId)))
                .and_where(Party::Id.equals(AssetParams::kind_id())),
            AssetKind::Emote | AssetKind::Sticker => q
                .from(
                    Emotes::inner_join_table::<UserAssetFiles>().on(UserAssetFiles::AssetId.equals(Emotes::AssetId))
                    .inner_join_table::<Files>().on(Files::Id.equals(UserAssetFiles::FileId)))
                .and_where(Emotes::Id.equals(AssetParams::kind_id()))
                .and_where(AssetParams::asset_id().is_not_null()) // make sure this parameter is used

            // TODO: Also validate emotes/stickers for their emote/sticker flags
        };

        q
    }
}
