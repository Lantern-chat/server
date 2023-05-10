use sdk::models::*;
use thorn::pg::Json;

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

pub async fn get_full_self(state: &ServerState, user_id: Snowflake) -> Result<User, Error> {
    let db = state.db.read.get().await?;

    get_full_self_inner(state, user_id, &db).await
}

pub async fn get_full_self_inner(
    state: &ServerState,
    user_id: Snowflake,
    db: &db::pool::Object,
) -> Result<User, Error> {
    let row = db.query_one_cached_typed(|| q::query(), &[&user_id]).await?;

    use q::{PresenceColumns, ProfileColumns, UserColumns};

    Ok(User {
        id: user_id,
        username: row.try_get(UserColumns::username())?,
        discriminator: row.try_get(UserColumns::discriminator())?,
        flags: UserFlags::from_bits_truncate(row.try_get(UserColumns::flags())?),
        email: Some(row.try_get(UserColumns::email())?),
        presence: Some({
            let last_active = crate::backend::util::relative::approximate_relative_time(
                &state,
                user_id,
                row.try_get(UserColumns::last_active())?,
                None,
            );

            match row.try_get(PresenceColumns::updated_at())? {
                Some(updated_at) => UserPresence {
                    flags: UserPresenceFlags::from_bits_truncate_public(row.try_get(PresenceColumns::flags())?),
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
        preferences: { row.try_get::<_, Option<_>>(UserColumns::preferences())?.map(|v: Json<_>| v.0) },
        profile: match row.try_get(ProfileColumns::bits())? {
            None => Nullable::Null,
            Some(bits) => Nullable::Some(Arc::new(UserProfile {
                bits,
                extra: Default::default(),
                nick: row.try_get(ProfileColumns::nickname())?,
                avatar: encrypt_snowflake_opt(state, row.try_get(ProfileColumns::avatar_id())?).into(),
                banner: encrypt_snowflake_opt(state, row.try_get(ProfileColumns::banner_id())?).into(),
                status: row.try_get(ProfileColumns::custom_status())?,
                bio: row.try_get(ProfileColumns::biography())?,
            })),
        },
    })
}

mod q {
    pub use schema::*;
    pub use thorn::*;

    thorn::indexed_columns! {
        pub enum UserColumns {
            Users::Username,
            Users::Discriminator,
            Users::Flags,
            Users::Email,
            Users::Preferences,
            Users::LastActive,
        }

        pub enum PresenceColumns continue UserColumns {
            AggPresence::UpdatedAt,
            AggPresence::Flags,
            //AggPresence::Activity,
        }

        pub enum ProfileColumns continue PresenceColumns {
            Profiles::Bits,
            Profiles::Nickname,
            Profiles::AvatarId,
            Profiles::BannerId,
            Profiles::CustomStatus,
            Profiles::Biography,
        }
    }

    pub fn query() -> impl AnyQuery {
        Query::select()
            .cols(UserColumns::default())
            .cols(PresenceColumns::default())
            .cols(ProfileColumns::default())
            .from(
                Users::left_join_table::<Profiles>()
                    .on(Profiles::UserId.equals(Users::Id).and(Profiles::PartyId.is_null()))
                    .left_join_table::<AggPresence>()
                    .on(AggPresence::UserId.equals(Users::Id)),
            )
            .and_where(Users::Id.equals(Var::of(Users::Id)))
            .and_where(Users::DeletedAt.is_null())
    }
}
