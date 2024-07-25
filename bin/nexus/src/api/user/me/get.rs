use sdk::models::*;
use thorn::pg::Json;

use crate::{prelude::*, util::encrypted_asset::encrypt_snowflake_opt};

pub async fn get_full_self(state: &ServerState, user_id: UserId) -> Result<User, Error> {
    let db = state.db.read.get().await?;

    get_full_self_inner(state, user_id, &db).await
}

pub async fn get_full_self_inner(state: &ServerState, user_id: UserId, db: &db::Object) -> Result<User, Error> {
    #[rustfmt::skip]
    let row = db.query_one2(schema::sql! {
        SELECT
            Users.Username          AS @Username,
            Users.Discriminator     AS @Discriminator,
            Users.Flags             AS @Flags,
            Users.Email             AS @Email,
            Users.Preferences       AS @Preferences,
            Users.LastActive        AS @LastActive,
            AggPresence.UpdatedAt   AS @UpdatedAt,
            AggPresence.Flags       AS @PresenceFlags,
            Profiles.Bits           AS @ProfileBits,
            Profiles.Nickname       AS @Nickname,
            Profiles.AvatarId       AS @AvatarId,
            Profiles.BannerId       AS @BannerId,
            Profiles.CustomStatus   AS @CustomStatus,
            Profiles.Biography      AS @Biography
        FROM LiveUsers AS Users
            LEFT JOIN Profiles ON Profiles.UserId = Users.Id AND Profiles.PartyId IS NULL
            LEFT JOIN AggPresence ON AggPresence.UserId = Users.Id
        WHERE Users.Id = #{&user_id as Users::Id}
    }).await?;

    Ok(User {
        id: user_id,
        username: row.username()?,
        discriminator: row.discriminator()?,
        flags: UserFlags::from_bits_truncate(row.flags()?),
        email: Some(row.email()?),
        presence: Some({
            let last_active =
                crate::util::relative::approximate_relative_time(state, user_id, row.last_active()?, None);

            match row.updated_at()? {
                Some(updated_at) => UserPresence {
                    flags: UserPresenceFlags::from_bits_truncate_public(row.presence_flags()?),
                    last_active,
                    updated_at: Some(updated_at),
                    activity: None,
                },
                None => UserPresence {
                    flags: UserPresenceFlags::empty(),
                    last_active,
                    updated_at: None,
                    activity: None,
                },
            }
        }),
        preferences: { row.preferences::<Option<_>>()?.map(|v: Json<_>| v.0) },
        profile: match row.profile_bits()? {
            None => Nullable::Null,
            Some(bits) => Nullable::Some(Arc::new(UserProfile {
                bits,
                extra: Default::default(),
                nick: row.nickname()?,
                avatar: encrypt_snowflake_opt(state, row.avatar_id()?).into(),
                banner: encrypt_snowflake_opt(state, row.banner_id()?).into(),
                status: row.custom_status()?,
                bio: row.biography()?,
            })),
        },
    })
}
