use futures::{Stream, StreamExt, TryStreamExt};

use db::pool::Client;
use schema::Snowflake;

use crate::{
    backend::{
        api::{auth::Authorization, SearchMode},
        util::encrypted_asset::encrypt_snowflake_opt,
    },
    Error, ServerState,
};

use sdk::models::*;

fn base_query() -> thorn::query::SelectQuery {
    use schema::*;
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
    db: &Client,
    party_id: SearchMode<'a>,
) -> Result<impl Stream<Item = Result<CustomEmote, Error>> + 'static, Error> {
    let stream = match party_id {
        SearchMode::Single(id) => db
            .query_stream_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    base_query().and_where(Emotes::PartyId.equals(Var::of(Party::Id)))
                },
                &[&id],
            )
            .await?
            .boxed(),
        SearchMode::Many(ids) => db
            .query_stream_cached_typed(
                || {
                    use schema::*;
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
