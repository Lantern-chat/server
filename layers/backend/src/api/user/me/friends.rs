use futures::{Stream, StreamExt};

use crate::{Authorization, Error, State};

use sdk::models::*;

pub async fn friends(
    state: State,
    auth: Authorization,
) -> Result<impl Stream<Item = Result<Friend, Error>>, Error> {
    let db = state.db.read.get().await?;

    let stream = db
        .query_stream_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .cols(&[
                        /*0*/ AggFriends::FriendId,
                        /*1*/ AggFriends::Note,
                        /*2*/ AggFriends::Flags,
                    ])
                    .cols(&[
                        /*3*/ Users::Username,
                        /*4*/ Users::Flags,
                        /*5*/ Users::Discriminator,
                        /*6*/ Users::CustomStatus,
                        /*7*/ Users::Biography,
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
                avatar: None,
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
