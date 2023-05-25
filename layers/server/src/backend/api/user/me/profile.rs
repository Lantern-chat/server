use sdk::{api::commands::user::UpdateUserProfileBody, models::*};

use crate::{
    backend::{
        asset::{maybe_add_asset, AssetMode},
        util::encrypted_asset::encrypt_snowflake,
    },
    Authorization, Error, ServerState,
};

pub async fn patch_profile(
    state: ServerState,
    auth: Authorization,
    mut profile: UpdateUserProfileBody,
    party_id: Option<Snowflake>,
) -> Result<UserProfile, Error> {
    {
        // TODO: Better errors here
        let config = state.config();
        if matches!(profile.status, Nullable::Some(ref status) if status.len() > config.user.max_custom_status_len)
        {
            return Err(Error::BadRequest);
        }

        if matches!(profile.bio, Nullable::Some(ref bio) if bio.len() > config.user.max_biography_len) {
            return Err(Error::BadRequest);
        }
    }

    let needs_to_do_work = !profile.avatar.is_undefined() || !profile.banner.is_undefined();

    let mut perms = Permissions::all();
    let db = state.db.read.get().await?;

    // check permissions and file ids before we try to adjust profile
    if needs_to_do_work {
        let mut avatar_id: Option<Snowflake> = None;
        let mut banner_id: Option<Snowflake> = None;

        // if party, check permissions at the same time as acquiring the file ids
        if party_id.is_some() {
            #[rustfmt::skip]
            let Some(row) = db.query_opt2(schema::sql! {
                SELECT
                    PartyMembers.Permissions1 AS @Permissions1,
                    PartyMembers.Permissions2 AS @Permissions2,
                    Profiles.AvatarFileId AS @AvatarId,
                    Profiles.BannerFileId AS @BannerId
                 FROM PartyMembers LEFT JOIN AggOriginalProfileFiles AS Profiles
                   ON Profiles.PartyId = PartyMembers.PartyId AND Profiles.UserId = PartyMembers.UserId
                WHERE PartyMembers.UserId = #{&auth.user_id as Users::Id}
                  AND PartyMembers.PartyId = #{&party_id as Party::Id}
            }).await? else {
                return Err(Error::Unauthorized);
            };

            perms = Permissions::from_i64(row.permissions1()?, row.permissions2()?);

            avatar_id = row.avatar_id()?;
            banner_id = row.banner_id()?;
        } else if let Some(row) = db
            .query_opt2(schema::sql! {
                SELECT
                    Profiles.AvatarFileId AS @AvatarId,
                    Profiles.BannerFileId AS @BannerId
                FROM AggOriginalProfileFiles AS Profiles
                WHERE Profiles.UserId = #{&auth.user_id as Users::Id}
                AND Profiles.PartyId IS NULL
            })
            .await?
        {
            avatar_id = row.avatar_id()?;
            banner_id = row.banner_id()?;
        }

        // No change, don't change
        if Nullable::from(avatar_id) == profile.avatar {
            profile.avatar = Nullable::Undefined;
        }

        if Nullable::from(banner_id) == profile.banner {
            profile.banner = Nullable::Undefined;
        }
    } else if party_id.is_some() {
        // TODO: Recombine the logic to merge with other query
        #[rustfmt::skip]
        let Some(row) = db.query_opt2(schema::sql! {
            SELECT
                PartyMembers.Permissions1 AS @Permissions1,
                PartyMembers.Permissions2 AS @Permissions2
            FROM PartyMembers
            WHERE PartyMembers.UserId = #{&auth.user_id as Users::Id}
                AND PartyMembers.PartyId = #{&party_id as Party::Id}
        }).await? else {
            return Err(Error::Unauthorized);
        };

        perms = Permissions::from_i64(row.permissions1()?, row.permissions2()?);
    }

    drop(db);

    if !perms.contains(Permissions::CHANGE_NICKNAME) && profile.nick.is_some() {
        return Err(Error::Unauthorized);
    }

    let (avatar_id, banner_id) = tokio::try_join!(
        maybe_add_asset(&state, AssetMode::Avatar, auth.user_id, profile.avatar),
        maybe_add_asset(&state, AssetMode::Banner, auth.user_id, profile.banner),
    )?;

    #[rustfmt::skip]
    let res = state.db.write.get().await?.execute2(schema::sql! {
        INSERT INTO Profiles (UserId, PartyId, Bits, AvatarId, BannerId, Nickname, CustomStatus, Biography) VALUES (
            #{&auth.user_id     as Users::Id},
            #{&party_id         as Party::Id},
            #{&profile.bits     as Profiles::Bits},
            #{&avatar_id        as Profiles::AvatarId},
            #{&banner_id        as Profiles::BannerId},
            #{&profile.nick     as Profiles::Nickname},
            #{&profile.status   as Profiles::CustomStatus},
            #{&profile.bio      as Profiles::Biography}
        )
        ON CONFLICT (Profiles./UserId, COALESCE(Profiles./PartyId, 1)) DO UPDATE SET
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
        nick: profile.nick,
        status: profile.status,
        bio: profile.bio,
        avatar: avatar_id.map(|id| encrypt_snowflake(&state, id)),
        banner: banner_id.map(|id| encrypt_snowflake(&state, id)),
    })
}
