use futures::{Stream, StreamExt};

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

use sdk::models::*;

mod friend_query {
    pub use schema::*;
    pub use thorn::*;

    indexed_columns! {
        pub enum FriendColumns {
            AggFriends::FriendId,
            AggFriends::Note,
            AggFriends::Flags,
        }

        pub enum UserColumns continue FriendColumns {
            Users::Username,
            Users::Flags,
            Users::Discriminator,
        }

        pub enum ProfileColumns continue UserColumns {
            Profiles::AvatarId,
            Profiles::Bits,
            Profiles::CustomStatus,
        }
    }
}

pub async fn friends(
    state: ServerState,
    auth: Authorization,
) -> Result<impl Stream<Item = Result<Friend, Error>>, Error> {
    let db = state.db.read.get().await?;

    let stream = db
        .query_stream_cached_typed(
            || {
                use friend_query::*;

                Query::select()
                    .cols(FriendColumns::default())
                    .cols(UserColumns::default())
                    .cols(ProfileColumns::default())
                    .from(
                        AggFriends::inner_join_table::<Users>()
                            .on(Users::Id.equals(AggFriends::FriendId))
                            .left_join_table::<Profiles>()
                            .on(Profiles::UserId
                                .equals(AggFriends::FriendId)
                                .and(Profiles::PartyId.is_null())),
                    )
                    .and_where(AggFriends::UserId.equals(Var::of(Users::Id)))
            },
            &[&auth.user_id],
        )
        .await?;

    use friend_query::{FriendColumns, ProfileColumns, UserColumns};

    Ok(stream.map(move |res| match res {
        Err(e) => Err(e.into()),
        Ok(row) => Ok(Friend {
            note: row.try_get(FriendColumns::note())?,
            flags: FriendFlags::from_bits_truncate(row.try_get(FriendColumns::flags())?),
            user: User {
                id: row.try_get(FriendColumns::friend_id())?,
                username: row.try_get(UserColumns::username())?,
                flags: UserFlags::from_bits_truncate(row.try_get(UserColumns::discriminator())?).publicize(),
                discriminator: row.try_get(UserColumns::discriminator())?,
                email: None,
                preferences: None,
                profile: match row.try_get(ProfileColumns::bits())? {
                    None => None,
                    Some(bits) => Some(UserProfile {
                        bits,
                        avatar: encrypt_snowflake_opt(&state, row.try_get(ProfileColumns::avatar_id())?),
                        status: row.try_get(ProfileColumns::custom_status())?,
                        banner: None,
                        bio: None,
                    }),
                },
            },
        }),
    }))
}
