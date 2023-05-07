use std::{io::SeekFrom, str::FromStr, time::Instant};

use bytes::{Bytes, BytesMut};
use ftl::{
    fs::{bytes_range, Cond, Conditionals},
    *,
};

use filesystem::store::{CipherOptions, OpenMode};
use futures::FutureExt;
use headers::{
    AcceptRanges, ContentLength, ContentRange, ContentType, HeaderMap, HeaderMapExt, HeaderValue, IfModifiedSince,
    LastModified, Range,
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

    #[rustfmt::skip]
    let row = state.db.read.get().await?.query_opt2(schema::sql! {
        SELECT
            Files.Id    AS @Id,
            Files.Size  AS @Size,
            Files.Flags AS @Flags,
            Files.Nonce AS @Nonce,
            Files.Name  AS @Name,
            Files.Mime  AS @Mime
        FROM
            UserAssetFiles INNER JOIN Files ON Files.Id = UserAssetFiles.FileId

        // pick the correct constraints to match valid assets
        match kind {
            AssetKind::UserAvatar => {
                INNER JOIN Profiles ON Profiles.AvatarId = UserAssetFiles.AssetId
                WHERE Profiles.UserId = #{&kind_id as Profiles::UserId}
            }
            AssetKind::UserBanner => {
                INNER JOIN Profiles ON Profiles.BannerId = UserAssetFiles.AssetId
                WHERE Profiles.UserId = #{&kind_id as Profiles::UserId}
            }
            AssetKind::RoleAvatar => {
                INNER JOIN Roles ON Roles.AvatarId = UserAssetFiles.AssetId
                WHERE Roles.Id = #{&kind_id as Roles::Id}
            }
            AssetKind::RoomAvatar => {
                INNER JOIN Rooms ON Rooms.AvatarId = UserAssetFiles.AssetId
                WHERE Rooms.Id = #{&kind_id as Rooms::Id}
            }
            AssetKind::PartyAvatar => {
                INNER JOIN Party ON Party.AvatarId = UserAssetFiles.AssetId
                WHERE Party.Id = #{&kind_id as Party::Id}
            }
            AssetKind::PartyBanner => {
                INNER JOIN Party ON Party.BannerId = UserAssetFiles.AssetId
                WHERE Party.Id = #{&kind_id as Party::Id}
            }
            AssetKind::Emote | AssetKind::Sticker => {
                INNER JOIN Emotes ON Emotes.AssetId = UserAssetFiles.AssetId
                WHERE Emotes.Id = #{&kind_id as Emotes::Id}
            }
        }

        if !matches!(kind, AssetKind::Emote | AssetKind::Sticker) {
            AND UserAssetFiles.AssetId = #{&asset_id as UserAssetFiles::AssetId}
        }

        AND (UserAssetFiles.Flags & {AssetFlags::QUALITY.bits()}) >= LEAST(
            100, {AssetFlags::QUALITY.bits()} & #{&flags as UserAssetFiles::Flags}
        )
        AND (
            // if it has the requested format flags
            (UserAssetFiles.Flags & #{&flags as UserAssetFiles::Flags} & {AssetFlags::FORMATS.bits()}) != 0
            // or if it has no flags at all
            OR UserAssetFiles.Flags & {AssetFlags::FLAGS.bits()} = 0
        )
        // prioritize images by flags (animation, alpha), then by file size to pick the smallest
        ORDER BY UserAssetFiles.Flags & {AssetFlags::FLAGS.bits()} DESC, Files.Size ASC
        LIMIT 1
    }).await;

    let row = match row {
        Ok(None) => return Err(Error::NotFound),
        Ok(Some(row)) => row,
        Err(e) => return Err(e.into()),
    };

    let meta = ParsedFile {
        id: row.id()?,
        size: row.size()?,
        flags: schema::flags::FileFlags::from_bits_truncate(row.flags()?),
        nonce: row.nonce()?,
        name: row.name()?,
        mime: row.mime()?,
    };

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

    #[rustfmt::skip]
    let row = state.db.read.get().await?.query_opt2(schema::sql! {
        SELECT
            Files.Id    AS @Id,
            Files.Size  AS @Size,
            Files.Flags AS @Flags,
            Files.Nonce AS @Nonce,
            Files.Name  AS @Name,
            Files.Mime  AS @Mime
        FROM Attachments
            INNER JOIN Messages ON Attachments.MessageId = Messages.Id
            INNER JOIN Files ON Files.Id = Attachments.FileId
        WHERE
            Files.Id        = #{&file_id as Files::Id}
        AND Messages.RoomId = #{&room_id as Rooms::Id}
    }).await?;

    let Some(row) = row else { return Err(Error::NotFound); };

    let m = ParsedFile {
        id: row.id()?,
        size: row.size()?,
        flags: schema::flags::FileFlags::from_bits_truncate(row.flags()?),
        nonce: row.nonce()?,
        name: row.name()?,
        mime: row.mime()?,
    };

    if let Some(provided_name) = provided_name {
        if m.name != provided_name {
            log::debug!("{:?} != {:?}", m.name, provided_name);
            return Err(Error::NotFound);
        }
    }

    sendfile::send_file(state, m, is_head, download, range, last_modified, Some(start)).await
}

pub struct ParsedFile<'a> {
    pub id: Snowflake,
    pub size: i32,
    pub flags: schema::flags::FileFlags,
    pub nonce: i64,
    pub name: &'a str,
    pub mime: Option<&'a str>,
}
