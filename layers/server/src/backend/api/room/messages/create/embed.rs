use std::sync::{
    atomic::{AtomicI16, Ordering},
    Arc,
};

use futures::{
    stream::{FuturesUnordered, Stream, StreamExt},
    Future, FutureExt,
};
use hashbrown::HashSet;
use schema::SnowflakeExt;
use sdk::models::*;

use tokio::task::JoinHandle;

use crate::{Error, ServerState};

use md_utils::{Span, SpanList, SpanType};

use self::query::get_embed;

pub fn process_embeds(state: ServerState, msg_id: Snowflake, msg: &str, spans: &[Span]) {
    let mut position = 0;
    let max_embeds = state.config().message.max_embeds as i16;

    // for checking duplicates
    let mut urls = HashSet::new();

    let embed_tasks = FuturesUnordered::new();

    for span in spans {
        if span.kind() == SpanType::Url {
            let url = &msg[span.range()];

            if !urls.insert(url) {
                continue;
            }

            embed_tasks.push(state.queues.embed_processing.push(run_embed(
                state.clone(),
                msg_id,
                url.to_owned(),
                position,
            )));

            position += 1;

            // bail out after max embed limit reached
            if position == max_embeds {
                return;
            }
        }
    }

    tokio::spawn(after_embed(state, msg_id, embed_tasks));
}

pub async fn after_embed<R>(
    state: ServerState,
    msg_id: Snowflake,
    mut embed_tasks: FuturesUnordered<JoinHandle<Result<Result<(), Error>, R>>>,
) {
    let mut num_successful = 0;

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

    if let Err(e) = db.execute_cached_typed(|| query::update_message(), &[&msg_id]).await {
        log::error!("Could not update message {msg_id} after embed processing: {e}");
    }
}

pub async fn run_embed(state: ServerState, msg_id: Snowflake, url: String, position: i16) -> Result<(), Error> {
    let db = state.db.read.get().await?;

    let mut embed_id = None;
    let mut fetch = true;

    if let Some(row) = db.query_opt_cached_typed(|| get_embed(), &[&url]).await? {
        let existing_id: Snowflake = row.try_get(0)?;
        let expires: Timestamp = row.try_get(1)?;

        embed_id = Some(existing_id);
        fetch = expires <= Timestamp::now_utc();
    }

    drop(db); // free connection early. Need to reacquire a write connection anyway after fetching.

    let embed_id = embed_id.unwrap_or_else(Snowflake::now);

    let mut maybe_db = None;

    if fetch {
        let Some((expires, embed)) = state.services.embed.fetch(&state, url, None).await? else {
            return Ok(());
        };

        let db = state.db.write.get().await?;

        use thorn::pg::Json;

        db.execute_cached_typed(
            || query::insert_embed(),
            &[&embed_id, &embed.url(), &Json(&embed), &expires],
        )
        .await?;

        maybe_db = Some(db);
    }

    let db = match maybe_db {
        None => state.db.write.get().await?,
        Some(db) => db,
    };

    db.execute_cached_typed(|| query::insert_message_embed(), &[&embed_id, &msg_id, &position])
        .await?;

    Ok(())
}

mod query {
    use schema::*;
    use thorn::{conflict::ConflictAction, *};

    pub fn get_embed() -> impl AnyQuery {
        Query::select()
            .cols(&[Embeds::Id, Embeds::Expires])
            .from_table::<Embeds>()
            .and_where(Embeds::Url.equals(Var::of(Embeds::Url)))
            .limit_n(1)
    }

    pub fn insert_embed() -> impl AnyQuery {
        let id = Var::at(Embeds::Id, 1);
        let url = Var::at(Embeds::Url, 2);
        let embed = Var::at(Embeds::Embed, 3);
        let expires = Var::at(Embeds::Expires, 4);

        tables! {
            pub struct ExistingEmbed {
                Id: Embeds::Id,
            }

            pub struct TempEmbed {
                Id: Embeds::Id,
                Url: Embeds::Url,
                Embed: Embeds::Embed,
                Expires: Embeds::Expires,
            }
        }

        let new_embed = TempEmbed::as_query(Query::select().exprs([
            id.alias_to(TempEmbed::Id),
            url.clone().alias_to(TempEmbed::Url),
            embed.clone().alias_to(TempEmbed::Embed),
            expires.clone().alias_to(TempEmbed::Expires),
        ]));

        let existing = ExistingEmbed::as_query(
            Query::select()
                .expr(Embeds::Id.alias_to(ExistingEmbed::Id))
                .from_table::<Embeds>()
                .and_where(Embeds::Url.equals(url)),
        );

        Query::insert()
            .with(new_embed)
            .with(existing.exclude())
            .into::<Embeds>()
            .cols(&[Embeds::Id, Embeds::Url, Embeds::Embed, Embeds::Expires])
            .query(
                Query::select()
                    .expr(Builtin::coalesce((ExistingEmbed::Id, TempEmbed::Id)))
                    .cols(&[TempEmbed::Url, TempEmbed::Embed, TempEmbed::Expires])
                    .from(TempEmbed::left_join_table::<ExistingEmbed>().on(true.lit()))
                    .as_value(),
            )
            .on_conflict(
                [Embeds::Id],
                ConflictAction::DoUpdateSet(DoUpdate.set(Embeds::Embed, embed).set(Embeds::Expires, expires)),
            )
    }

    pub fn insert_message_embed() -> impl AnyQuery {
        Query::insert()
            .into::<MessageEmbeds>()
            .cols(&[MessageEmbeds::EmbedId, MessageEmbeds::MsgId, MessageEmbeds::Position])
            .values([
                Var::at(MessageEmbeds::EmbedId, 1),
                Var::at(MessageEmbeds::MsgId, 2),
                Var::at(MessageEmbeds::Position, 3),
            ])
    }

    pub fn update_message() -> impl AnyQuery {
        Query::update()
            .table::<Messages>()
            .and_where(Messages::Id.equals(Var::of(Messages::Id)))
            .set_default(Messages::UpdatedAt)
    }
}
