use hashbrown::hash_map::{Entry, HashMap};

use schema::Snowflake;

use crate::{
    ctrl::{auth::AuthToken, Error},
    ServerState,
};

#[derive(Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

use futures::{Stream, StreamExt};

use models::{AnyActivity, PartyMember, User, UserFlags, UserPresence, UserPresenceFlags};

pub async fn get_members(
    state: ServerState,
    party_id: Snowflake,
) -> Result<impl Stream<Item = Result<PartyMember, Error>>, Error> {
    let db = state.db.read.get().await?;

    let stream = db
        .query_stream_cached_typed(|| select_members2(), &[&party_id])
        .await?;

    Ok(stream.map(|row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => Ok({
            let user_id = row.try_get(1)?;
            PartyMember {
                nick: row.try_get(0)?,
                user: Some(User {
                    id: user_id,
                    username: row.try_get(2)?,
                    discriminator: row.try_get(3)?,
                    flags: UserFlags::from_bits_truncate(row.try_get(4)?).publicize(),
                    status: None,
                    bio: None,
                    email: None,
                    preferences: None,
                    avatar_id: None,
                }),
                presence: match row.try_get::<_, Option<chrono::NaiveDateTime>>(5)? {
                    None => None,
                    Some(updated_at) => Some(UserPresence {
                        updated_at: Some(crate::util::time::format_naivedatetime(updated_at)),
                        flags: UserPresenceFlags::from_bits_truncate(row.try_get(6)?),
                        activity: match row.try_get::<_, Option<serde_json::Value>>(7)? {
                            None => None,
                            Some(value) => Some(AnyActivity::Any(value)),
                        },
                    }),
                },
                roles: row.try_get(8)?,
            }
        }),
    }))
}

use thorn::*;

fn select_members2() -> impl AnyQuery {
    use schema::*;

    tables! {
        struct AggPresence {
            UserId: UserPresence::UserId,
            UpdatedAt: UserPresence::UpdatedAt,
            Flags: UserPresence::Flags,
            Activity: UserPresence::Activity,
        }
    }

    Query::with()
        .with(
            AggPresence::as_query(
                Query::select()
                    .distinct()
                    .on(UserPresence::UserId)
                    .cols(&[
                        UserPresence::UserId,
                        UserPresence::UpdatedAt,
                        UserPresence::Flags,
                        UserPresence::Activity,
                    ])
                    .from_table::<UserPresence>()
                    .order_by(UserPresence::UserId.ascending())
                    .order_by(UserPresence::UpdatedAt.descending()),
            )
            .exclude(),
        )
        .select()
        .cols(&[/* 0 */ PartyMember::Nickname])
        .cols(&[
            /* 1 */ Users::Id,
            /* 2 */ Users::Username,
            /* 3 */ Users::Discriminator,
            /* 4 */ Users::Flags,
        ])
        .cols(&[
            /* 5 */ AggPresence::UpdatedAt,
            /* 6 */ AggPresence::Flags,
            /* 7 */ AggPresence::Activity,
        ])
        .expr(
            /* 8 */
            Call::custom("ARRAY").args(
                Query::select()
                    .col(RoleMembers::RoleId)
                    .from_table::<RoleMembers>()
                    .and_where(RoleMembers::UserId.equals(Users::Id))
                    .as_value(),
            ),
        )
        .from(
            AggPresence::right_join(
                PartyMember::inner_join_table::<Users>().on(Users::Id.equals(PartyMember::UserId)),
            )
            .on(AggPresence::UserId.equals(PartyMember::UserId)),
        )
        .and_where(PartyMember::PartyId.equals(Var::of(Party::Id)))
}
