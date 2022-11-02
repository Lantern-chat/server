use futures::{future::Either, TryFutureExt};
use sdk::{api::commands::user::UpdateUserProfileBody, models::*};

use crate::{Authorization, Error, ServerState};

pub async fn get_profile(
    state: ServerState,
    auth: Authorization,
    user_id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<UserProfile, Error> {
    let db = state.db.read.get().await?;

    let base_row_future = async {
        db.query_opt_cached_typed(
            || {
                use q::*;
                q::base_query()
                    .and_where(Profiles::UserId.equals(Var::of(Profiles::UserId)))
                    .and_where(Profiles::PartyId.is_null())
            },
            &[&user_id],
        )
        .await
    };

    let party_row_future = async {
        if party_id.is_none() {
            return Ok(None);
        }

        db.query_opt_cached_typed(
            || {
                use q::*;
                q::base_query()
                    .and_where(Profiles::UserId.equals(Var::of(Profiles::UserId)))
                    .and_where(Profiles::PartyId.equals(Var::of(Profiles::PartyId)))
            },
            &[&user_id, &party_id],
        )
        .await
    };

    let (base_row, party_row) = tokio::try_join!(base_row_future, party_row_future)?;

    Ok(match (base_row, party_row) {
        (None, None) => UserProfile::default(),
        (Some(row), None) | (None, Some(row)) => q::raw_profile_to_public(&state, q::parse_profile(row)?),
        (Some(base_row), Some(party_row)) => {
            let mut party = q::parse_profile(party_row)?;

            let q::RawProfile {
                avatar,
                banner,
                status,
                bio,
                nick,
                bits,
            } = q::parse_profile(base_row)?;

            party.bits = {
                let mut avatar_bits = party.bits;
                let mut banner_bits = party.bits;

                // if there is no party avatar, copy over the base avatar bits
                if party.avatar.is_none() {
                    avatar_bits = bits;
                }

                // likewise
                if !party.bits.contains(UserProfileBits::OVERRIDE_COLOR) {
                    banner_bits = bits;
                }

                (avatar_bits & UserProfileBits::AVATAR_ROUNDNESS)
                    | (banner_bits & (UserProfileBits::OVERRIDE_COLOR | UserProfileBits::PRIMARY_COLOR))
            };

            // these are eequivalent to `COALESCE(party.*, base.*)`
            party.nick = party.nick.or(nick);
            party.avatar = party.avatar.or(avatar);
            party.banner = party.banner.or(banner);
            party.status = party.status.or(status);
            party.bio = party.bio.or(bio);

            q::raw_profile_to_public(&state, party)
        }
    })
}

mod q {
    pub use schema::*;
    pub use thorn::*;

    thorn::indexed_columns! {
        pub enum ProfileColumns {
            Profiles::Nickname,
            Profiles::AvatarId,
            Profiles::BannerId,
            Profiles::Bits,
            Profiles::CustomStatus,
            Profiles::Biography,
        }
    }

    thorn::params! {
        pub struct ProfileParams {
            pub user_id: Snowflake = Profiles::UserId,
            pub party_id: Option<Snowflake> = Profiles::PartyId,
        }
    }

    pub fn base_query() -> query::SelectQuery {
        Query::select()
            .from_table::<Profiles>()
            .cols(ProfileColumns::default())
    }

    use sdk::models::{Snowflake, UserProfile, UserProfileBits};
    use smol_str::SmolStr;

    use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, ServerState};

    pub struct RawProfile {
        pub bits: UserProfileBits,
        pub nick: Option<SmolStr>,
        pub avatar: Option<Snowflake>,
        pub banner: Option<Snowflake>,
        pub status: Option<SmolStr>,
        pub bio: Option<SmolStr>,
    }

    pub fn parse_profile(row: db::pg::Row) -> Result<RawProfile, db::pg::Error> {
        Ok(RawProfile {
            bits: row.try_get(ProfileColumns::bits())?,
            nick: row.try_get(ProfileColumns::nickname())?,
            avatar: row.try_get(ProfileColumns::avatar_id())?,
            banner: row.try_get(ProfileColumns::banner_id())?,
            status: row.try_get(ProfileColumns::custom_status())?,
            bio: row.try_get(ProfileColumns::biography())?,
        })
    }

    pub fn raw_profile_to_public(state: &ServerState, raw: RawProfile) -> UserProfile {
        UserProfile {
            bits: raw.bits,
            extra: Default::default(),
            nick: raw.nick.into(),
            avatar: encrypt_snowflake_opt(&state, raw.avatar).into(),
            banner: encrypt_snowflake_opt(&state, raw.banner).into(),
            status: raw.status.into(),
            bio: raw.bio.into(),
        }
    }
}
