use hashbrown::hash_map::{Entry, HashMap};

use schema::{Snowflake, SnowflakeExt};
use util::hex::HexidecimalInt;

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Error, ServerState};

use futures::{Stream, StreamExt};

use sdk::models::*;

// TODO: Add cursor-based pagination
pub async fn get_members(
    state: ServerState,
    party_id: Snowflake,
    user_id: Snowflake,
) -> Result<impl Stream<Item = Result<PartyMember, Error>>, Error> {
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
        .query_stream_cached_typed(|| query::select_members(), &[&party_id])
        .await?;

    Ok(stream.map(move |row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => parse_member(row, &state),
    }))
}

pub fn parse_member(row: db::Row, state: &ServerState) -> Result<PartyMember, Error> {
    use query::{MemberColumns, ProfileColumns, UserColumns};

    Ok(PartyMember {
        user: Some(User {
            id: row.try_get(UserColumns::id())?,
            username: row.try_get(UserColumns::username())?,
            discriminator: row.try_get(UserColumns::discriminator())?,
            flags: UserFlags::from_bits_truncate(row.try_get(UserColumns::flags())?).publicize(),
            email: None,
            preferences: None,
            profile: match row.try_get(ProfileColumns::bits())? {
                None => Nullable::Null,
                Some(bits) => Nullable::Some(UserProfile {
                    bits,
                    avatar: encrypt_snowflake_opt(state, row.try_get(ProfileColumns::avatar_id())?).into(),
                    banner: Nullable::Undefined,
                    status: row.try_get(ProfileColumns::custom_status())?,
                    bio: Nullable::Undefined,
                }),
            },
        }),
        nick: row.try_get(MemberColumns::nickname())?,
        presence: match row.try_get(UserColumns::presence_updated_at())? {
            None => None,
            Some(updated_at) => Some(UserPresence {
                updated_at: Some(updated_at),
                flags: UserPresenceFlags::from_bits_truncate(row.try_get(UserColumns::presence_flags())?),
                activity: match row.try_get(UserColumns::presence_activity())? {
                    None => None,
                    Some(value) => Some(AnyActivity::Any(value)),
                },
            }),
        },
        roles: row.try_get(MemberColumns::role_ids())?,
        flags: None,
    })
}

pub(crate) mod query {
    use schema::*;
    use thorn::*;

    pub use super::parse_member;

    indexed_columns! {
        pub enum UserColumns {
            AggUsers::Id,
            AggUsers::Discriminator,
            AggUsers::Username,
            AggUsers::Flags,
            AggUsers::PresenceFlags,
            AggUsers::PresenceUpdatedAt,
            AggUsers::PresenceActivity,
        }

        pub enum MemberColumns continue UserColumns {
            AggMembers::Nickname,
            AggMembers::JoinedAt,
            AggMembers::RoleIds,
        }

        pub enum ProfileColumns continue MemberColumns {
            AggProfiles::AvatarId,
            AggProfiles::Bits,
            AggProfiles::CustomStatus,
        }
    }

    pub fn select_members() -> query::SelectQuery {
        Query::select()
            .cols(UserColumns::default())
            .cols(MemberColumns::default())
            .cols(ProfileColumns::default())
            .from(
                AggUsers::inner_join_table::<AggMembers>()
                    .on(AggMembers::UserId.equals(AggUsers::Id))
                    .left_join_table::<AggProfiles>()
                    .on(AggProfiles::UserId
                        .equals(AggUsers::Id)
                        // profiles.party_id is allowed to be NULL, just not false
                        .and(AggProfiles::PartyId.equals(AggMembers::PartyId).is_not_false())),
            )
            .and_where(AggMembers::PartyId.equals(Var::of(Party::Id)))
            // not banned
            .and_where(AggMembers::Flags.bit_and(1i16.lit()).equals(0i16.lit()))
    }
}
