use sdk::models::*;
use thorn::pg::Json;

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

pub async fn get_full(state: &ServerState, user_id: Snowflake) -> Result<User, Error> {
    mod self_query {
        pub use schema::*;
        pub use thorn::*;

        indexed_columns! {
            pub enum UserColumns {
                Users::Username,
                Users::Discriminator,
                Users::Flags,
                Users::Email,
                Users::Preferences,
            }

            pub enum ProfileColumns continue UserColumns {
                Profiles::AvatarId,
                Profiles::BannerId,
                Profiles::Bits,
                Profiles::CustomStatus,
                Profiles::Biography,
            }
        }
    }

    let db = state.db.read.get().await?;

    let row = db
        .query_one_cached_typed(
            || {
                use self_query::*;

                Query::select()
                    .cols(UserColumns::default())
                    .cols(ProfileColumns::default())
                    .from(
                        Users::left_join_table::<Profiles>().on(Profiles::UserId
                            .equals(Users::Id)
                            .and(Profiles::PartyId.is_null())),
                    )
                    .and_where(Users::Id.equals(Var::of(Users::Id)))
            },
            &[&user_id],
        )
        .await?;

    use self_query::{ProfileColumns, UserColumns};

    Ok(User {
        id: user_id,
        username: row.try_get(UserColumns::username())?,
        discriminator: row.try_get(UserColumns::discriminator())?,
        flags: UserFlags::from_bits_truncate(row.try_get(UserColumns::flags())?),
        email: Some(row.try_get(UserColumns::email())?),
        preferences: {
            row.try_get::<_, Option<_>>(UserColumns::preferences())?
                .map(|v: Json<_>| v.0)
        },
        profile: match row.try_get(ProfileColumns::bits())? {
            None => Nullable::Null,
            Some(bits) => Nullable::Some(UserProfile {
                bits,
                avatar: encrypt_snowflake_opt(&state, row.try_get(ProfileColumns::avatar_id())?).into(),
                banner: encrypt_snowflake_opt(&state, row.try_get(ProfileColumns::banner_id())?).into(),
                status: row.try_get(ProfileColumns::custom_status())?,
                bio: row.try_get(ProfileColumns::biography())?,
            }),
        },
    })
}
