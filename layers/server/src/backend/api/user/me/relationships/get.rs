use futures::{Stream, StreamExt};

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

use sdk::models::*;

pub async fn get_relationships(
    state: ServerState,
    auth: Authorization,
) -> Result<impl Stream<Item = Result<Relationship, Error>>, Error> {
    let db = state.db.read.get().await?;

    let stream = db.query_stream_cached_typed(|| q::query(), &[&auth.user_id]).await?;

    use q::{AssocColumns, ProfileColumns, RelColumns, UserColumns};

    Ok(stream.map(move |res| match res {
        Err(e) => Err(e.into()),
        Ok(row) => Ok({
            // NOTE: The query only returns users we have not blocked, so no additional
            // logic is necessary to merge associated with blocked
            let associated: bool = row.try_get(AssocColumns::associated())?;
            let rel = row.try_get(RelColumns::rel_a())?;
            let user_id = row.try_get(RelColumns::friend_id())?;

            Relationship {
                note: row.try_get(RelColumns::note())?,
                since: row.try_get(RelColumns::updated_at())?,
                rel,
                pending: {
                    let rel_b: UserRelationship = row.try_get(RelColumns::rel_b())?;

                    matches!((rel, rel_b), (UserRelationship::Friend, UserRelationship::None))
                },
                user: User {
                    id: user_id,
                    username: row.try_get(UserColumns::username())?,
                    flags: UserFlags::from_bits_truncate_public(row.try_get(UserColumns::flags())?),
                    discriminator: row.try_get(UserColumns::discriminator())?,
                    presence: match associated {
                        // such is the case where you are sending a friend request to a non-associated user
                        // still can't view their presence until it's accepted or a party is shared
                        false => None,
                        true => {
                            let last_active = crate::backend::util::relative::approximate_relative_time(
                                &state,
                                user_id,
                                row.try_get(UserColumns::last_active())?,
                                None,
                            );

                            match row.try_get(UserColumns::presence_updated_at())? {
                                Some(updated_at) => Some(UserPresence {
                                    flags: UserPresenceFlags::from_bits_truncate_public(
                                        row.try_get(UserColumns::presence_flags())?,
                                    ),
                                    last_active,
                                    updated_at: Some(updated_at),
                                    activity: None,
                                }),
                                None => Some(UserPresence {
                                    flags: UserPresenceFlags::empty(),
                                    last_active,
                                    updated_at: None,
                                    activity: None,
                                }),
                            }
                        }
                    },
                    email: None,
                    preferences: None,
                    profile: match row.try_get(ProfileColumns::bits())? {
                        None => Nullable::Null,
                        Some(bits) => Nullable::Some(UserProfile {
                            bits,
                            extra: Default::default(),
                            nick: match associated {
                                true => row.try_get(ProfileColumns::nickname())?,
                                false => Nullable::Undefined,
                            },
                            status: match associated {
                                true => row.try_get(ProfileColumns::custom_status())?,
                                false => Nullable::Undefined,
                            },
                            avatar: encrypt_snowflake_opt(&state, row.try_get(ProfileColumns::avatar_id())?)
                                .into(),
                            banner: Nullable::Undefined,
                            bio: Nullable::Undefined,
                        }),
                    },
                },
            }
        }),
    }))
}

mod q {
    pub use schema::*;
    use sdk::models::UserRelationship;
    pub use thorn::*;

    thorn::tables! {
        pub struct AggAssociated {
            Associated: Type::BOOL,
        }
    }

    thorn::indexed_columns! {
        pub enum RelColumns {
            AggRelationships::FriendId,
            AggRelationships::UpdatedAt,
            AggRelationships::RelA,
            AggRelationships::RelB,
            AggRelationships::Note,
        }

        pub enum UserColumns continue RelColumns {
            AggUsers::Username,
            AggUsers::Flags,
            AggUsers::Discriminator,
            AggUsers::LastActive,
            AggUsers::PresenceFlags,
            AggUsers::PresenceUpdatedAt,
            //AggUsers::PresenceActivity,
        }

        pub enum ProfileColumns continue UserColumns {
            Profiles::Nickname,
            Profiles::AvatarId,
            Profiles::Bits,
            Profiles::CustomStatus,
        }

        pub enum AssocColumns continue ProfileColumns {
            AggAssociated::Associated
        }
    }

    pub fn query() -> impl AnyQuery {
        Query::select()
            .cols(RelColumns::default())
            .cols(UserColumns::default())
            .cols(ProfileColumns::default())
            // AssocColumns
            .expr(
                // TODO: Maybe avoid the inner agg_relationships query
                Query::select()
                    .from_table::<AggUserAssociations>()
                    .expr(1.lit())
                    .and_where(AggUserAssociations::UserId.equals(AggRelationships::UserId))
                    .and_where(AggUserAssociations::OtherId.equals(AggRelationships::FriendId))
                    .exists(),
            )
            .from(
                AggRelationships::inner_join_table::<AggUsers>()
                    .on(AggUsers::Id.equals(AggRelationships::FriendId))
                    .left_join_table::<Profiles>()
                    .on(Profiles::UserId
                        .equals(AggRelationships::FriendId)
                        .and(Profiles::PartyId.is_null())),
            )
            .and_where(AggRelationships::UserId.equals(Var::of(AggUsers::Id)))
            // where the other user has not blocked this one
            .and_where(AggRelationships::RelB.less_than((UserRelationship::Blocked as i8).lit()))
            .and_where(
                // where b < 2 && !((a == b) && (a == 0)), meaning it should filter None relationships, only
                // allowing friends, pending, and users blocked by ourselves
                AggRelationships::RelA
                    .equals(AggRelationships::RelB)
                    .and(AggRelationships::RelA.equals(0.lit()))
                    .is_false(),
            )
    }
}
