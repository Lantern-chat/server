use hashbrown::hash_map::{Entry, HashMap};

use schema::{Snowflake, SnowflakeExt};

use crate::{
    ctrl::{auth::AuthToken, Error},
    util::hex::HexidecimalInt,
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

    Ok(stream.map(move |row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => Ok({
            let user_id = row.try_get(0)?;
            PartyMember {
                nick: row.try_get(10)?,
                user: Some(User {
                    id: user_id,
                    username: row.try_get(2)?,
                    discriminator: row.try_get(1)?,
                    flags: UserFlags::from_bits_truncate(row.try_get(3)?).publicize(),
                    status: row.try_get(5)?,
                    bio: row.try_get(4)?,
                    email: None,
                    preferences: None,
                    avatar: {
                        let avatar_id: Option<Snowflake> = row.try_get(9)?;

                        match avatar_id {
                            None => None,
                            Some(id) => {
                                let encrypted_id = HexidecimalInt(id.encrypt(state.config.sf_key));

                                Some(encrypted_id.to_string())
                            }
                        }
                    },
                }),
                presence: match row.try_get::<_, Option<chrono::NaiveDateTime>>(7)? {
                    None => None,
                    Some(updated_at) => Some(UserPresence {
                        updated_at: Some(crate::util::time::format_naivedatetime(updated_at)),
                        flags: UserPresenceFlags::from_bits_truncate(row.try_get(6)?),
                        activity: match row.try_get::<_, Option<serde_json::Value>>(8)? {
                            None => None,
                            Some(value) => Some(AnyActivity::Any(value)),
                        },
                    }),
                },
                roles: row.try_get(12)?,
            }
        }),
    }))
}

use thorn::*;

fn select_members2() -> impl AnyQuery {
    use schema::*;

    Query::select()
        .from(AggUsers::inner_join_table::<AggMembers>().on(AggMembers::UserId.equals(AggUsers::Id)))
        .cols(&[
            /* 0*/ AggUsers::Id,
            /* 1*/ AggUsers::Discriminator,
            /* 2*/ AggUsers::Username,
            /* 3*/ AggUsers::UserFlags,
            /* 4*/ AggUsers::Biography,
            /* 5*/ AggUsers::CustomStatus,
            /* 6*/ AggUsers::PresenceFlags,
            /* 7*/ AggUsers::PresenceUpdatedAt,
            /* 8*/ AggUsers::PresenceActivity,
        ])
        .expr(
            /* 9*/ Builtin::coalesce((AggMembers::AvatarId, AggUsers::AvatarId)),
        )
        .cols(&[
            /*10*/ AggMembers::Nickname,
            /*11*/ AggMembers::JoinedAt,
            /*12*/ AggMembers::RoleIds,
        ])
        .and_where(AggMembers::PartyId.equals(Var::of(Party::Id)))

    /*

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

        */
}
