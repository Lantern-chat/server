use futures::{Stream, StreamExt};

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

use sdk::models::*;

pub async fn friends(
    state: ServerState,
    auth: Authorization,
) -> Result<impl Stream<Item = Result<Friend, Error>>, Error> {
    let db = state.db.read.get().await?;

    let stream = db
        .query_stream_cached_typed(|| q::query(), &[&auth.user_id])
        .await?;

    use q::{AssocColumns, FriendColumns, ProfileColumns, UserColumns};

    Ok(stream.map(move |res| match res {
        Err(e) => Err(e.into()),
        Ok(row) => Ok({
            let associated = row.try_get(AssocColumns::associated())?;

            Friend {
                note: row.try_get(FriendColumns::note())?,
                flags: row.try_get(FriendColumns::flags())?,
                since: row.try_get(FriendColumns::updated_at())?,
                user: User {
                    id: row.try_get(FriendColumns::friend_id())?,
                    username: row.try_get(UserColumns::username())?,
                    flags: UserFlags::from_bits_truncate_public(row.try_get(UserColumns::flags())?),
                    discriminator: row.try_get(UserColumns::discriminator())?,
                    presence: None, // TODO
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
    pub use thorn::*;

    thorn::tables! {
        pub struct AggAssociated {
            Associated: Type::BOOL,
        }
    }

    thorn::indexed_columns! {
        pub enum FriendColumns {
            AggFriends::FriendId,
            AggFriends::Note,
            AggFriends::Flags,
            AggFriends::UpdatedAt,
        }

        pub enum UserColumns continue FriendColumns {
            Users::Username,
            Users::Flags,
            Users::Discriminator,
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

    // TODO: Also join on visibility and adjust hidden columns
    pub fn query() -> impl AnyQuery {
        Query::select()
            .cols(FriendColumns::default())
            .cols(UserColumns::default())
            .cols(ProfileColumns::default())
            // AssocColumns
            .expr(
                Query::select()
                    .from_table::<AggUserAssociations>()
                    .expr(1.lit())
                    .and_where(AggUserAssociations::UserId.equals(AggFriends::UserId))
                    .and_where(AggUserAssociations::OtherId.equals(AggFriends::FriendId))
                    .exists(),
            )
            .from(
                AggFriends::inner_join_table::<Users>()
                    .on(Users::Id.equals(AggFriends::FriendId))
                    .left_join_table::<Profiles>()
                    .on(Profiles::UserId
                        .equals(AggFriends::FriendId)
                        .and(Profiles::PartyId.is_null())),
            )
            .and_where(AggFriends::UserId.equals(Var::of(Users::Id)))
    }
}
