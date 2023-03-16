use hashbrown::hash_map::{Entry, HashMap};

use schema::{Snowflake, SnowflakeExt};
use util::hex::HexidecimalInt;

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

use futures::{FutureExt, Stream, StreamExt};

use sdk::models::*;

/// `full` indicates it should also include extra profile information
pub async fn get_one_anonymouse(
    state: &ServerState,
    db: &db::pool::Client,
    party_id: Snowflake,
    member_id: Snowflake,
    full: bool,
) -> Result<PartyMember, Error> {
    use q::{Parameters, Params};

    let params = Params {
        user_id: None,
        party_id,
        member_id: Some(member_id),
    };

    let stmt = match full {
        true => db.prepare_cached_typed(|| q::query(true, true, true)).boxed(),
        false => db.prepare_cached_typed(|| q::query(true, false, true)).boxed(),
    };

    let row = db.query_one(&stmt.await?, &params.as_params()).await?;

    parse_member(row, state, full)
}

/// `full` indicates it should also include extra profile information
pub async fn get_one(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
    member_id: Snowflake,
    full: bool,
) -> Result<PartyMember, Error> {
    let db = state.db.read.get().await?;

    use q::{Parameters, Params};

    let params = Params {
        user_id: Some(auth.user_id),
        party_id,
        member_id: Some(member_id),
    };

    let stmt = match full {
        true => db.prepare_cached_typed(|| q::query(true, true, false)).boxed(),
        false => db.prepare_cached_typed(|| q::query(true, false, false)).boxed(),
    };

    let row = db.query_one(&stmt.await?, &params.as_params()).await?;

    parse_member(row, &state, full)
}

pub async fn get_many(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
) -> Result<impl Stream<Item = Result<PartyMember, Error>>, Error> {
    let db = state.db.read.get().await?;

    use q::{Parameters, Params};

    let params = Params {
        user_id: Some(auth.user_id),
        party_id,
        member_id: None,
    };

    let stream = db
        .query_stream_cached_typed(|| q::query(false, false, false), &params.as_params())
        .await?;

    Ok(stream.map(move |row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => parse_member(row, &state, false),
    }))
}

mod q {
    use super::Cursor;

    use schema::flags::MemberFlags;
    pub use schema::*;
    pub use thorn::*;

    use db::Row;
    use sdk::models::UserRelationship;

    thorn::tables! {
        pub struct TempIsMember {
            IsMember: Type::BOOL,
        }
    }

    thorn::params! {
        pub struct Params {
            pub user_id: Option<Snowflake> = Users::Id,
            pub party_id: Snowflake = PartyMember::PartyId,
            pub member_id: Option<Snowflake> = PartyMember::UserId,
        }
    }

    indexed_columns! {
        pub enum MemberColumns {
            AggMembersFull::UserId,
            AggMembersFull::Discriminator,
            AggMembersFull::Username,
            AggMembersFull::LastActive,
            AggMembersFull::UserFlags,
            AggMembersFull::PresenceFlags,
            AggMembersFull::PresenceUpdatedAt,
            AggMembersFull::Nickname,
            //AggMembersFull::MemberFlags,
            AggMembersFull::JoinedAt,
            AggMembersFull::ProfileBits,
            AggMembersFull::AvatarId,
            AggMembersFull::CustomStatus,
            AggMembersFull::RoleIds,
            AggMembersFull::PresenceActivity,
        }

        pub enum RelColumns continue MemberColumns {
            AggRelationships::RelB,
        }

        pub enum ExtraColumns continue RelColumns {
            AggMembersFull::BannerId,
            AggMembersFull::Biography,
        }
    }

    pub fn query(single: bool, full: bool, anonymous: bool) -> impl thorn::AnyQuery {
        let is_member = Query::select()
            .from_table::<PartyMember>()
            .and_where(PartyMember::UserId.equals(Params::user_id()))
            .and_where(PartyMember::PartyId.equals(Params::party_id()))
            .exists();

        let mut q = Query::select()
            .and_where(AggMembersFull::PartyId.equals(Params::party_id()))
            .and_where(AggMembersFull::MemberFlags.has_no_bits(MemberFlags::BANNED.bits().lit()))
            .cols(MemberColumns::default());

        // RelColumns::RelB
        q = match anonymous {
            true => q.expr(0.lit()),
            false => q.expr(Builtin::coalesce((AggRelationships::RelB, 0.lit()))),
        };

        q = match anonymous {
            true => q.from_table::<AggMembersFull>(),
            false => q.and_where(is_member).from(
                AggMembersFull::left_join_table::<AggRelationships>().on(AggRelationships::UserId
                    .equals(Params::user_id())
                    .and(AggRelationships::FriendId.equals(AggMembersFull::UserId))),
            ),
        };

        if full {
            q = q.cols(ExtraColumns::default());
        }

        q = match single {
            true => q.and_where(AggMembersFull::UserId.equals(Params::member_id())),
            false => q.and_where(Params::member_id().is_null()),
        };

        q
    }
}

fn parse_member(row: db::Row, state: &ServerState, full: bool) -> Result<PartyMember, Error> {
    use q::{ExtraColumns, MemberColumns, RelColumns};

    let rel_b: UserRelationship = row.try_get(RelColumns::rel_b())?;

    // if the other user has not blocked us
    let is_friendly = rel_b < UserRelationship::Blocked;

    let user_id = row.try_get(MemberColumns::user_id())?;

    Ok(PartyMember {
        user: User {
            id: user_id,
            username: row.try_get(MemberColumns::username())?,
            discriminator: row.try_get(MemberColumns::discriminator())?,
            flags: UserFlags::from_bits_truncate_public(row.try_get(MemberColumns::user_flags())?),
            presence: match is_friendly {
                false => None,
                true => {
                    let last_active = match full {
                        false => None,
                        true => crate::backend::util::relative::approximate_relative_time(
                            state,
                            user_id,
                            row.try_get(MemberColumns::last_active())?,
                            None,
                        ),
                    };

                    Some(match row.try_get(MemberColumns::presence_updated_at())? {
                        None => UserPresence {
                            flags: UserPresenceFlags::empty(),
                            last_active,
                            updated_at: None,
                            activity: None,
                        },
                        Some(updated_at) => UserPresence {
                            last_active,
                            updated_at: Some(updated_at),
                            flags: UserPresenceFlags::from_bits_truncate_public(
                                row.try_get(MemberColumns::presence_flags())?,
                            ),
                            activity: match row.try_get(MemberColumns::presence_activity())? {
                                None => None,
                                Some(value) => Some(AnyActivity::Any(value)),
                            },
                        },
                    })
                }
            },
            email: None,
            preferences: None,
            profile: match row.try_get(MemberColumns::profile_bits())? {
                None => Nullable::Null,
                Some(bits) => Nullable::Some(UserProfile {
                    bits,
                    extra: Default::default(),
                    nick: match is_friendly {
                        true => row.try_get(MemberColumns::nickname())?,
                        false => Nullable::Undefined,
                    },
                    avatar: encrypt_snowflake_opt(state, row.try_get(MemberColumns::avatar_id())?).into(),
                    banner: match full && is_friendly {
                        true => encrypt_snowflake_opt(state, row.try_get(ExtraColumns::banner_id())?).into(),
                        false => Nullable::Undefined,
                    },
                    status: match is_friendly {
                        true => row.try_get(MemberColumns::custom_status())?,
                        false => Nullable::Undefined,
                    },
                    bio: match full && is_friendly {
                        true => row.try_get(ExtraColumns::biography())?,
                        false => Nullable::Undefined,
                    },
                }),
            },
        },
        partial: PartialPartyMember {
            joined_at: row.try_get(MemberColumns::joined_at())?,
            roles: row.try_get(MemberColumns::role_ids())?,
            flags: None,
        },
    })
}
