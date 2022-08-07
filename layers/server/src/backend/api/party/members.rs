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
    use query::MemberColumns;

    Ok(PartyMember {
        user: Some(User {
            id: row.try_get(MemberColumns::user_id())?,
            username: row.try_get(MemberColumns::username())?,
            discriminator: row.try_get(MemberColumns::discriminator())?,
            flags: UserFlags::from_bits_truncate_public(row.try_get(MemberColumns::user_flags())?),
            email: None,
            preferences: None,
            profile: match row.try_get(MemberColumns::profile_bits())? {
                None => Nullable::Null,
                Some(bits) => Nullable::Some(UserProfile {
                    bits,
                    avatar: encrypt_snowflake_opt(state, row.try_get(MemberColumns::avatar_id())?).into(),
                    banner: Nullable::Undefined,
                    status: row.try_get(MemberColumns::custom_status())?,
                    bio: Nullable::Undefined,
                }),
            },
        }),
        nick: row.try_get(MemberColumns::nickname())?,
        presence: match row.try_get(MemberColumns::presence_updated_at())? {
            None => None,
            Some(updated_at) => Some(UserPresence {
                updated_at: Some(updated_at),
                flags: UserPresenceFlags::from_bits_truncate_public(
                    row.try_get(MemberColumns::presence_flags())?,
                ),
                activity: match row.try_get(MemberColumns::presence_activity())? {
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
        pub enum MemberColumns {
            AggMembersFull::UserId,
            AggMembersFull::Discriminator,
            AggMembersFull::Username,
            AggMembersFull::UserFlags,
            AggMembersFull::PresenceFlags,
            AggMembersFull::PresenceUpdatedAt,
            AggMembersFull::Nickname,
            //AggMembersFull::MemberFlags,
            AggMembersFull::JoinedAt,
            AggMembersFull::AvatarId,
            AggMembersFull::ProfileBits,
            AggMembersFull::CustomStatus,
            AggMembersFull::RoleIds,
            AggMembersFull::PresenceActivity,
        }
    }

    pub fn select_members() -> query::SelectQuery {
        Query::select()
            .cols(MemberColumns::default())
            .from_table::<AggMembersFull>()
            .and_where(AggMembersFull::PartyId.equals(Var::of(Party::Id)))
            // not banned
            .and_where(AggMembersFull::MemberFlags.bit_and(1i16.lit()).equals(0i16.lit()))
    }
}
