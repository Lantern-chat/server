use futures::stream::{FuturesUnordered, StreamExt};
use hashbrown::HashSet;
use schema::SnowflakeExt;
use sdk::models::*;

use crate::prelude::*;

use md_utils::{Span, SpanType};

pub fn process_embeds(state: ServerState, msg_id: Snowflake, msg: &str, spans: &[Span]) {
    let mut position = 0;
    let max_embeds = state.config().shared.max_embeds as i16;

    // for checking duplicates
    let mut urls = HashSet::new();

    let embed_tasks = FuturesUnordered::new();

    for span in spans {
        if span.kind() == SpanType::Url {
            let url = &msg[span.range()];

            if !urls.insert(url) {
                continue;
            }

            log::debug!("Starting task of fetching {url}");

            embed_tasks.push(state.queues.embed_processing.push(run_embed(
                state.clone(),
                msg_id,
                url.to_owned(),
                position,
                md_utils::is_spoilered(spans, span.start()),
            )));

            position += 1;

            // bail out after max embed limit reached
            if position == max_embeds {
                return;
            }
        }
    }

    tokio::spawn(async move {
        let mut num_successful = 0;

        let mut embed_tasks = embed_tasks;

        while let Some(res) = embed_tasks.next().await {
            let res = match res {
                Ok(inner) => inner,
                Err(e) => {
                    log::error!("Error executing embed task: {e}");
                    continue;
                }
            };

            let res = match res {
                Ok(inner) => inner,
                Err(_) => {
                    log::error!("Error queuing embed task");
                    continue;
                }
            };

            if let Err(e) = res {
                log::warn!("Error fetching embed: {e}");
            } else {
                num_successful += 1;
            }
        }

        if num_successful == 0 {
            return;
        }

        let db = match state.db.write.get().await {
            Ok(db) => db,
            Err(e) => {
                log::error!("Cannot get database connection to finalize embed update: {e}");
                return;
            }
        };

        let update_message = db.execute2(schema::sql! {
            UPDATE Messages SET (UpdatedAt) = DEFAULT WHERE Messages.Id = #{&msg_id as Messages::Id}
        });

        if let Err(e) = update_message.await {
            log::error!("Could not update message {msg_id} after embed processing: {e}");
        }
    });
}

pub async fn run_embed(
    state: ServerState,
    msg_id: Snowflake,
    url: String,
    position: i16,
    spoilered: bool,
) -> Result<(), Error> {
    let db = state.db.read.get().await?;

    let mut embed_id = None;
    let mut refresh = true;

    #[rustfmt::skip]
    let existing = db.query_opt2(schema::sql! {
        SELECT Embeds.Id AS @EmbedId, Embeds.Expires AS @Expires
        FROM Embeds WHERE Embeds.Url = #{&url as Embeds::Url} LIMIT 1
    });

    if let Some(row) = existing.await? {
        let existing_id: Snowflake = row.embed_id()?;
        let expires: Timestamp = row.expires()?;

        embed_id = Some(existing_id);
        refresh = expires <= Timestamp::now_utc();
    }

    drop(db); // free connection early. Need to reacquire a write connection anyway after fetching.

    let mut embed_id = embed_id.unwrap_or_else(|| state.sf.gen());
    let flags = spoilered.then_some(EmbedFlags::SPOILER);

    // if we happen to get a db object after fetching, keep it around to reuse it
    let mut maybe_db = None;

    if refresh {
        let Some((expires, embed)) = state.services.embed.fetch(&state, url.clone(), None).await? else {
            return Ok(());
        };

        let db = state.db.write.get().await?;

        let embed = thorn::pg::Json(&embed);

        // NOTE: The original URL should be used, not embed.url(), do to potential odd duplicates
        #[rustfmt::skip]
        let row = db.query_one2(schema::sql! {
            tables! {
                pub struct TempEmbed {
                    Id: Embeds::Id,
                }
                pub struct ExistingEmbed {
                    Id: Embeds::Id,
                }
            };

            WITH ExistingEmbed AS (
                SELECT Embeds.Id AS ExistingEmbed.Id
                FROM Embeds WHERE Embeds.Url = #{&url as Embeds::Url}
            ),
            TempEmbed AS (
                SELECT #{&embed_id as Embeds::Id} AS TempEmbed.Id
            )
            INSERT INTO Embeds (Id, Url, Embed, Expires) (
                SELECT
                    COALESCE(ExistingEmbed.Id, TempEmbed.Id),
                    #{&url      as Embeds::Url},
                    #{&embed    as Embeds::Embed},
                    #{&expires  as Embeds::Expires}
                FROM TempEmbed LEFT JOIN ExistingEmbed ON TRUE
            )
            ON CONFLICT (Embeds./Id) DO UPDATE Embeds SET (Embed, Expires) = (
                #{&embed    as Embeds::Embed},
                #{&expires  as Embeds::Expires}
            )
            RETURNING Embeds.Id AS @EmbedId
        }).await?;

        // if another embed of the exact same url was found during insertion,
        // we should reuse its ID returned here
        embed_id = row.embed_id()?;

        maybe_db = Some(db);
    }

    let db = match maybe_db {
        None => state.db.write.get().await?,
        Some(db) => db,
    };

    db.execute2(schema::sql! {
        INSERT INTO MessageEmbeds (EmbedId, MsgId, Position, Flags) VALUES (
            #{&embed_id as MessageEmbeds::EmbedId},
            #{&msg_id   as MessageEmbeds::MsgId},
            #{&position as MessageEmbeds::Position},
            #{&flags    as MessageEmbeds::Flags}
        )
    })
    .await?;

    Ok(())
}
