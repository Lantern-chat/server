use futures::{Stream, StreamExt};

use db::Snowflake;
use models::*;

use crate::{ctrl::Error, ServerState};

pub async fn get_rooms(
    state: ServerState,
    party_id: Snowflake,
) -> Result<impl Stream<Item = Result<Room, Error>>, Error> {
    let stream = state
        .read_db()
        .await
        .query_stream_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Rooms>()
                    .cols(&[
                        Rooms::Id,
                        Rooms::Name,
                        Rooms::Topic,
                        Rooms::Flags,
                        Rooms::AvatarId,
                        Rooms::SortOrder,
                        Rooms::ParentId,
                    ])
                    .and_where(Rooms::DeletedAt.is_null())
                    .and_where(Rooms::PartyId.equals(Var::of(Party::Id)))
            },
            &[&party_id],
        )
        .await?;

    Ok(stream.map(|row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => Ok(unimplemented!()),
    }))
}
