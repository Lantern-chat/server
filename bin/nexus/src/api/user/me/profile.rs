use sdk::{api::commands::user::UpdateUserProfileBody, models::*};

use crate::{
    asset::{maybe_add_asset, AssetMode},
    prelude::*,
    util::encrypted_asset::encrypt_snowflake,
};

pub async fn patch_profile(
    state: ServerState,
    auth: Authorization,
    party_id: Option<PartyId>,
    profile: &Archived<UpdateUserProfileBody>,
) -> Result<UserProfile, Error> {
    {
        // TODO: Better errors here
        let config = state.config();
        if matches!(profile.status, Nullable::Some(ref status) if status.len() > config.shared.max_status_length) {
            return Err(Error::BadRequest);
        }

        if matches!(profile.bio, Nullable::Some(ref bio) if bio.len() > config.shared.max_bio_length) {
            return Err(Error::BadRequest);
        }
    }

    let has_assets = profile.avatar.is_some() || profile.banner.is_some();

    let mut perms = Permissions::all();
    let mut old_avatar_id: Option<FileId> = None;
    let mut old_banner_id: Option<FileId> = None;

    if party_id.is_some() {
        // for a party profile, we at least need the permissions, but also fetch old asset files if needed
        #[rustfmt::skip]
        let Some(row) = state.db.read.get().await?.query_opt2(schema::sql! {
            type Profiles = AggOriginalProfileFiles; // hacky

            SELECT
                PartyMembers.Permissions1 AS @Permissions1,
                PartyMembers.Permissions2 AS @Permissions2,
                if has_assets { Profiles.AvatarFileId } else { NULL } AS @AvatarFileId,
                if has_assets { Profiles.BannerFileId } else { NULL } AS @BannerFileId
            FROM PartyMembers if has_assets {
                LEFT JOIN AggOriginalProfileFiles AS Profiles
                ON Profiles.PartyId = PartyMembers.PartyId AND Profiles.UserId = PartyMembers.UserId
            }
            WHERE PartyMembers.UserId = #{auth.user_id_ref() as Users::Id}
              AND PartyMembers.PartyId = #{&party_id as Party::Id}
        }).await? else {
            return Err(Error::Unauthorized);
        };

        perms = Permissions::from_i64(row.permissions1()?, row.permissions2()?);

        old_avatar_id = row.avatar_file_id()?;
        old_banner_id = row.banner_file_id()?;
    } else if has_assets {
        // old asset files for non-party profiles are only necessary if they're being replaced
        #[rustfmt::skip]
        let row = state.db.read.get().await?.query_opt2(schema::sql! {
            SELECT
                Profiles.AvatarFileId AS @AvatarId,
                Profiles.BannerFileId AS @BannerId
            FROM AggOriginalProfileFiles AS Profiles
            WHERE Profiles.UserId = #{auth.user_id_ref() as Users::Id} AND Profiles.PartyId IS NULL
        }).await?;

        if let Some(row) = row {
            old_avatar_id = row.avatar_id()?;
            old_banner_id = row.banner_id()?;
        }
    }

    if !perms.contains(Permissions::CHANGE_NICKNAME) && profile.nick.is_some() {
        return Err(Error::Unauthorized);
    }

    let (avatar_id, banner_id) = {
        let mut new_avatar = profile.avatar;
        let mut new_banner = profile.banner;

        if has_assets {
            // No change, don't change
            if Nullable::<FileId>::from(old_avatar_id) == profile.avatar {
                new_avatar = Nullable::Undefined;
            }

            if Nullable::<FileId>::from(old_banner_id) == profile.banner {
                new_banner = Nullable::Undefined;
            }
        }

        tokio::try_join!(
            maybe_add_asset(&state, AssetMode::Avatar, auth.user_id(), new_avatar),
            maybe_add_asset(&state, AssetMode::Banner, auth.user_id(), new_banner),
        )?
    };

    #[rustfmt::skip]
    let res = state.db.write.get().await?.execute2(schema::sql! {
        INSERT INTO Profiles (UserId, PartyId, Bits, AvatarId, BannerId, Nickname, CustomStatus, Biography) VALUES (
            #{auth.user_id_ref()    as Users::Id},
            #{&party_id             as Party::Id},
            #{&profile.bits         as Profiles::Bits},
            #{&avatar_id            as Profiles::AvatarId},
            #{&banner_id            as Profiles::BannerId},
            #{&profile.nick         as Profiles::Nickname},
            #{&profile.status       as Profiles::CustomStatus},
            #{&profile.bio          as Profiles::Biography}
        )
        ON CONFLICT (Profiles./UserId, Profiles./PartyId) DO UPDATE SET
            if !avatar_id.is_undefined()      { Profiles./AvatarId = #{&avatar_id as Profiles::AvatarId}, }
            if !banner_id.is_undefined()      { Profiles./BannerId = #{&banner_id as Profiles::BannerId}, }
            if !profile.nick.is_undefined()   { Profiles./Nickname = #{&profile.nick as Profiles::Nickname}, }
            if !profile.status.is_undefined() { Profiles./CustomStatus = #{&profile.status as Profiles::CustomStatus}, }
            if !profile.bio.is_undefined()    { Profiles./Biography = #{&profile.bio as Profiles::Biography}, }

            Profiles./Bits = #{&profile.bits as Profiles::Bits}
    }).await?;

    if res == 0 {
        return Err(Error::InternalErrorStatic("Unknown error setting profile"));
    }

    Ok(UserProfile {
        bits: profile.bits,
        extra: Default::default(),
        nick: simple_de(&profile.nick),
        status: simple_de(&profile.status),
        bio: simple_de(&profile.bio),
        avatar: avatar_id.map(|id| encrypt_snowflake(&state, id)),
        banner: banner_id.map(|id| encrypt_snowflake(&state, id)),
    })
}
