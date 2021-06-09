use futures::{Stream, StreamExt, TryStreamExt};

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, Error, SearchMode},
    ServerState,
};

use models::*;

fn base_query() -> thorn::query::SelectQuery {
    use db::schema::*;
    use thorn::*;

    Query::select().from_table::<Emotes>().cols(&[
        Emotes::Id,
        Emotes::PartyId,
        Emotes::FileId,
        Emotes::Name,
        Emotes::Flags,
        Emotes::AspectRatio,
    ])
}

pub async fn get_custom_emotes_raw<'a>(
    state: &ServerState,
    party_id: SearchMode<'a>,
) -> Result<impl Stream<Item = Result<CustomEmote, Error>> + 'static, Error> {
    let client = state.read_db().await;

    let stream = match party_id {
        SearchMode::Single(id) => client
            .query_stream_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    base_query().and_where(Emotes::PartyId.equals(Var::of(Party::Id)))
                },
                &[&id],
            )
            .await?
            .boxed(),
        SearchMode::Many(ids) => client
            .query_stream_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    base_query().and_where(Emotes::PartyId.equals(Builtin::any(Var::of(SNOWFLAKE_ARRAY))))
                },
                &[&ids],
            )
            .await?
            .boxed(),
    };

    Ok(stream.map(|row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => Ok(CustomEmote {
            id: row.try_get(0)?,
            party_id: row.try_get(1)?,
            file: row.try_get(2)?,
            name: row.try_get(3)?,
            flags: EmoteFlags::from_bits_truncate(row.try_get(4)?),
            aspect_ratio: row.try_get(5)?,
        }),
    }))
}
