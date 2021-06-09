use futures::{Stream, StreamExt};

use crate::ctrl::{auth::Authorization, Error};
use crate::ServerState;

use models::*;

pub async fn friends(
    state: ServerState,
    auth: Authorization,
) -> Result<impl Stream<Item = Result<Friend, Error>>, Error> {
    let stream = state
        .db
        .read
        .get()
        .await?
        .query_stream_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .cols(&[AggFriends::FriendId, AggFriends::Note, AggFriends::Flags])
                    .cols(&[
                        Users::Username,
                        Users::Flags,
                        Users::Discriminator,
                        Users::CustomStatus,
                        Users::Biography,
                    ])
                    .from(AggFriends::inner_join_table::<Users>().on(Users::Id.equals(AggFriends::FriendId)))
                    .and_where(AggFriends::UserId.equals(Var::of(Users::Id)))
            },
            &[&auth.user_id],
        )
        .await?;

    Ok(stream.map(|res| match res {
        Err(e) => Err(e.into()),
        Ok(row) => Ok(Friend {
            note: row.try_get(1)?,
            flags: FriendFlags::from_bits_truncate(row.try_get(2)?),
            user: User {
                id: row.try_get(0)?,
                avatar_id: None,
                username: row.try_get(3)?,
                flags: UserFlags::from_bits_truncate(row.try_get(4)?).publicize(),
                discriminator: row.try_get(5)?,
                status: row.try_get(6)?,
                bio: row.try_get(7)?,
                email: None,
                preferences: None,
            },
        }),
    }))
}
