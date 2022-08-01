use futures::{future::Either, TryFutureExt};
use sdk::{api::commands::user::UpdateUserProfileBody, models::*};

use crate::{
    backend::{api::party::members::query::ProfileColumns, util::encrypted_asset::encrypt_snowflake_opt},
    Authorization, Error, ServerState,
};

pub async fn get_profile(
    state: ServerState,
    auth: Authorization,
    user_id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<UserProfile, Error> {
    let db = state.db.read.get().await?;

    pub mod profile_query {
        pub use schema::*;
        pub use thorn::*;

        thorn::indexed_columns! {
            pub enum ProfileColumns {
                AggProfiles::AvatarId,
                AggProfiles::BannerId,
                AggProfiles::Bits,
                AggProfiles::CustomStatus,
                AggProfiles::Biography,
            }
        }
    }

    let row = db
        .query_opt_cached_typed(
            || {
                use profile_query::*;

                Query::select()
                    .from_table::<AggProfiles>()
                    .cols(ProfileColumns::default())
                    .and_where(AggProfiles::UserId.equals(Var::of(AggProfiles::UserId)))
                    .and_where(
                        AggProfiles::PartyId
                            .equals(Var::of(AggProfiles::PartyId))
                            .is_not_false(),
                    )
            },
            &[&user_id, &party_id],
        )
        .await?;

    let row = match row {
        Some(row) => row,
        None => return Ok(UserProfile::default()),
    };

    use profile_query::ProfileColumns;

    Ok(UserProfile {
        bits: row.try_get(ProfileColumns::bits())?,
        avatar: encrypt_snowflake_opt(&state, row.try_get(ProfileColumns::avatar_id())?).into(),
        banner: encrypt_snowflake_opt(&state, row.try_get(ProfileColumns::banner_id())?).into(),
        status: row.try_get(ProfileColumns::custom_status())?,
        bio: row.try_get(ProfileColumns::biography())?,
    })
}
