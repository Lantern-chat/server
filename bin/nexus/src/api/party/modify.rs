use crate::asset::{maybe_add_asset, AssetMode};
use crate::prelude::*;

use sdk::api::commands::all::{PatchParty, PatchPartyForm};
use sdk::models::*;

pub async fn modify_party(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<PatchParty>,
) -> Result<Party, Error> {
    let party_id = cmd.party_id.into();
    let form = &cmd.body;

    if *form == PatchPartyForm::default() {
        return Err(Error::BadRequest);
    }

    {
        // TODO: Better errors
        let config = state.config();
        if matches!(form.description, Nullable::Some(ref desc) if !config.shared.party_description_length.contains(&desc.len()))
        {
            return Err(Error::BadRequest);
        }

        if matches!(form.name.as_ref(), Some(name) if !schema::validation::validate_name(name, config.shared.party_name_length.clone()))
        {
            return Err(Error::InvalidName);
        }
    }

    let has_assets = form.avatar.is_some() || form.banner.is_some();

    #[rustfmt::skip]
    let Some(row) = state.db.read.get().await?.query_opt2(schema::sql! {
        type AvatarAsset = UserAssets;
        type BannerAsset = UserAssets;

        SELECT
            PartyMembers.Permissions1 AS @Permissions1,
            PartyMembers.Permissions2 AS @Permissions2,
            if has_assets { AvatarAsset.FileId } else { NULL } AS @AvatarFileId,
            if has_assets { BannerAsset.FileId } else { NULL } AS @BannerFileId
        FROM Party
            INNER JOIN PartyMembers ON PartyMembers.PartyId = Party.Id
        if has_assets {
            LEFT JOIN UserAssets AS AvatarAsset ON AvatarAsset.Id = Party.AvatarId
            LEFT JOIN UserAssets AS BannerAsset ON BannerAsset.Id = Party.BannerId
        }
        WHERE Party.Id = #{&party_id as Party::Id}
          AND PartyMembers.UserId = #{auth.user_id_ref() as Users::Id}
    }).await? else {
        return Err(Error::Unauthorized);
    };

    let perms = Permissions::from_i64(row.permissions1()?, row.permissions2()?);

    if !perms.contains(Permissions::MANAGE_PARTY) {
        return Err(Error::Unauthorized);
    }

    let (avatar_id, banner_id) = {
        let mut new_avatar = form.avatar;
        let mut new_banner = form.banner;

        if has_assets {
            let old_avatar_id: Nullable<FileId> = row.avatar_file_id()?;
            let old_banner_id: Nullable<FileId> = row.banner_file_id()?;

            if old_avatar_id == form.avatar {
                new_avatar = Nullable::Undefined;
            }

            if old_banner_id == form.banner {
                new_banner = Nullable::Undefined;
            }
        }

        tokio::try_join!(
            maybe_add_asset(&state, AssetMode::Avatar, auth.user_id(), new_avatar.map(Into::into)),
            maybe_add_asset(&state, AssetMode::Banner, auth.user_id(), new_banner.map(Into::into)),
        )?
    };

    let set_room = form.default_room.is_some();

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    #[rustfmt::skip]
    let res = t.execute2(schema::sql! {
        tables! {
            struct TempDefaultRoom {
                Id: Rooms::Id,
            }
        };

        if set_room {
            // verify the room is within this party
            WITH TempDefaultRoom AS (
                SELECT Rooms.Id AS TempDefaultRoom.Id
                FROM LiveRooms AS Rooms WHERE Rooms.PartyId = #{&party_id as Party::Id}
            )
        }

        UPDATE Party SET
            if form.name.is_some()              { Party./Name        = #{&form.name as Party::Name}, }
            if !form.description.is_undefined() { Party./Description = #{&form.description as Party::Description}, }
            if !avatar_id.is_undefined()        { Party./AvatarId    = #{&avatar_id as Party::AvatarId}, }
            if !banner_id.is_undefined()        { Party./BannerId    = #{&avatar_id as Party::BannerId}, }
            if set_room                         { Party./DefaultRoom = TempDefaultRoom.Id, }

            Party./Flags = COALESCE(#{&form.flags as Party::Flags}, Party./Flags)
        if set_room { FROM TempDefaultRoom }
        WHERE Party.Id = #{&party_id as Party::Id}
    }).await?;

    if res != 1 {
        t.rollback().await?;

        return Err(Error::InternalErrorStatic("Unable to update party"));
    }

    t.commit().await?;

    crate::api::party::get::get_party(state, auth, party_id).await
}
