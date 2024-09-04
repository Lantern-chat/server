use crate::{prelude::*, rpc::SearchMode};

use sdk::models::*;

pub async fn get_custom_emotes_raw<'a, DB: db::AnyClient>(
    db: &DB,
    party_id: SearchMode<'a>,
) -> Result<impl Stream<Item = Result<CustomEmote, Error>> + 'static, Error> {
    let stream = db
        .query_stream2(schema::sql! {
            SELECT
                Emotes.Id           AS @_,
                Emotes.PartyId      AS @_,
                Emotes.AssetId      AS @_,
                Emotes.Name         AS @_,
                Emotes.Flags        AS @_,
                Emotes.AspectRatio  AS @_
            FROM Emotes WHERE match party_id {
                SearchMode::Single(ref id) => { Emotes.PartyId =     #{id  as SNOWFLAKE} },
                SearchMode::Many(ref ids)  => { Emotes.PartyId = ANY(#{ids as SNOWFLAKE_ARRAY}) },
            }
        })
        .await?;

    Ok(stream.map(|row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => Ok(CustomEmote {
            id: row.emotes_id()?,
            party_id: row.emotes_party_id()?,
            asset: row.emotes_asset_id()?,
            name: row.emotes_name()?,
            flags: row.emotes_flags()?,
            aspect_ratio: row.emotes_aspect_ratio()?,
        }),
    }))
}
