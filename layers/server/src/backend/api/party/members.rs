use hashbrown::hash_map::{Entry, HashMap};

use schema::{Snowflake, SnowflakeExt};
use util::hex::HexidecimalInt;

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Error, ServerState};

use futures::{Stream, StreamExt};

use sdk::models::{AnyActivity, PartyMember, User, UserFlags, UserPresence, UserPresenceFlags};

pub async fn get_members<'a>(
    state: &'a ServerState,
    party_id: Snowflake,
    user_id: Snowflake,
) -> Result<impl Stream<Item = Result<PartyMember, Error>> + 'a, Error> {
    let db = state.db.read.get().await?;

    let is_member = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<PartyMember>()
                    .and_where(PartyMember::PartyId.equals(Var::of(Party::Id)))
                    .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
            },
            &[&party_id, &user_id],
        )
        .await?;

    if is_member.is_none() {
        return Err(Error::NotFound);
    }

    let stream = db
        .query_stream_cached_typed(|| select_members2(), &[&party_id])
        .await?;

    Ok(stream.map(move |row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => Ok(PartyMember {
            user: Some(User {
                id: row.try_get(0)?,
                username: row.try_get(2)?,
                discriminator: row.try_get(1)?,
                flags: UserFlags::from_bits_truncate(row.try_get(3)?).publicize(),
                status: row.try_get(5)?,
                bio: row.try_get(4)?,
                email: None,
                preferences: None,
                avatar: encrypt_snowflake_opt(&state, row.try_get(9)?),
            }),
            nick: row.try_get(10)?,
            presence: match row.try_get::<_, Option<_>>(7)? {
                None => None,
                Some(updated_at) => Some(UserPresence {
                    updated_at: Some(updated_at),
                    flags: UserPresenceFlags::from_bits_truncate(row.try_get(6)?),
                    activity: match row.try_get::<_, Option<serde_json::Value>>(8)? {
                        None => None,
                        Some(value) => Some(AnyActivity::Any(value)),
                    },
                }),
            },
            roles: row.try_get(12)?,
            flags: None,
        }),
    }))
}

use thorn::*;

pub(crate) fn select_members2() -> query::SelectQuery {
    use schema::*;

    Query::select()
        .from(AggUsers::inner_join_table::<AggMembers>().on(AggMembers::UserId.equals(AggUsers::Id)))
        .cols(&[
            /* 0*/ AggUsers::Id,
            /* 1*/ AggUsers::Discriminator,
            /* 2*/ AggUsers::Username,
            /* 3*/ AggUsers::Flags,
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
        .and_where(
            // not banned
            AggMembers::Flags.bit_and(1i16.lit()).equals(0i16.lit()),
        )

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
