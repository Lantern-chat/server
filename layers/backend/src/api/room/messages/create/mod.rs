use std::borrow::Cow;

use futures::FutureExt;
use schema::{Snowflake, SnowflakeExt};
use smol_str::SmolStr;

use crate::{cache::permission_cache::PermMute, Authorization, Error, State};

use sdk::models::*;

pub mod embed;
pub mod slash;
pub mod trim;

use sdk::api::commands::room::CreateMessageBody;

/// Returns an `Option<Message>` because slash-commands may not actually create a message
pub async fn create_message(
    state: State,
    auth: Authorization,
    room_id: Snowflake,
    body: CreateMessageBody,
) -> Result<Option<Message>, Error> {
    // fast-path for if the perm_cache does contain a value, otherwise defer until content is checked
    let perm = match state.perm_cache.get(auth.user_id, room_id).await {
        Some(PermMute { perm, .. }) => {
            if !perm.contains(RoomPermissions::SEND_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            Some(perm)
        }
        None => None,
    };

    let trimmed_content = body.content.trim();

    // if empty but not containing attachments
    if trimmed_content.is_empty() && body.attachments.is_empty() {
        return Err(Error::BadRequest);
    }

    // do full trimming
    let trimmed_content = trim::trim_message(&state, &trimmed_content)?;

    // since acquiring the database connection may be expensive,
    // defer it until we need it, such as if the permissions cache didn't have a value
    let mut maybe_db = None;

    let perm = match perm {
        Some(perm) => perm,
        None => {
            let db = state.db.write.get().await?;

            let perm = crate::api::perm::get_room_permissions(&db, auth.user_id, room_id).await?;

            if !perm.contains(RoomPermissions::SEND_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            maybe_db = Some(db);

            perm
        }
    };

    let msg_id = Snowflake::now();

    // check this before acquiring database connection
    if !body.attachments.is_empty() && !perm.contains(RoomPermissions::ATTACH_FILES) {
        return Err(Error::Unauthorized);
    }

    // modify content before inserting it into the database
    let modified_content = match slash::process_slash(
        &trimmed_content,
        perm.room.contains(RoomPermissions::USE_SLASH_COMMANDS),
    ) {
        Ok(Some(content)) => content,
        Ok(None) => return Ok(None),
        Err(e) => return Err(e),
    };

    // message is good to go, so fire off the embed processing
    if !modified_content.is_empty() && perm.contains(RoomPermissions::EMBED_LINKS) {
        embed::process_embeds(msg_id, &modified_content);
    }

    // if we avoided getting a database connection until now, do it now
    let db = match maybe_db {
        Some(db) => db,
        None => state.db.write.get().await?,
    };

    let res = insert_message(db, state, auth, room_id, msg_id, &body, &modified_content)
        .boxed()
        .await;

    res.map(Option::Some)
}

pub(crate) async fn insert_message(
    mut db: db::pool::Object,
    state: State,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    body: &CreateMessageBody,
    content: &str,
) -> Result<Message, Error> {
    // TODO: Determine if repeatable-read is needed?
    let t = db.transaction().await?;

    // allow it to be null
    let content = if content.is_empty() { None } else { Some(content) };

    if let Some(parent_msg_id) = body.parent {
        let thread_id = Snowflake::now();

        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                let msg_id_var = Var::at(Messages::Id, 1);
                let user_id_var = Var::at(Users::Id, 2);
                let room_id_var = Var::at(Rooms::Id, 3);
                let parent_msg_id_var = Var::at(Messages::Id, 4);
                let thread_id_var = Var::at(Threads::Id, 5);
                let new_thread_flags_var = Var::at(Threads::Flags, 6);
                let content_var = Var::at(Messages::Content, 7);

                Query::insert()
                    .into::<Messages>()
                    .cols(&[
                        Messages::ThreadId,
                        Messages::Id,
                        Messages::UserId,
                        Messages::RoomId,
                        Messages::Content,
                    ])
                    .value(
                        Query::select()
                            .expr(Call::custom("lantern.create_thread").args((
                                thread_id_var,
                                parent_msg_id_var,
                                new_thread_flags_var,
                            )))
                            .as_value(),
                    )
                    .values([msg_id_var, user_id_var, room_id_var, content_var])
            },
            &[
                &msg_id,
                &auth.user_id,
                &room_id,
                &parent_msg_id,
                &thread_id,
                &ThreadFlags::empty().bits(), // TODO
                &content,
            ],
        )
        .await?;
    } else {
        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::insert()
                    .into::<Messages>()
                    .cols(&[
                        Messages::Id,
                        Messages::UserId,
                        Messages::RoomId,
                        Messages::Content,
                    ])
                    .values([
                        Var::of(Messages::Id),
                        Var::of(Messages::UserId),
                        Var::of(Messages::RoomId),
                        Var::of(Messages::Content),
                    ])
            },
            &[&msg_id, &auth.user_id, &room_id, &content],
        )
        .await?;
    }

    if !body.attachments.is_empty() {
        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                tables! {
                    struct AggIds {
                        Id: Files::Id,
                    }
                }

                let msg_id_var = Var::at(Messages::Id, 1);
                let att_id_var = Var::at(SNOWFLAKE_ARRAY, 2);

                Query::with()
                    .with(
                        AggIds::as_query(
                            Query::select().expr(Call::custom("UNNEST").arg(att_id_var).alias_to(AggIds::Id)),
                        )
                        .exclude(),
                    )
                    .insert()
                    .into::<Attachments>()
                    .cols(&[Attachments::FileId, Attachments::MessageId])
                    .query(
                        Query::select()
                            .col(AggIds::Id)
                            .expr(msg_id_var)
                            .from_table::<AggIds>()
                            .as_value(),
                    )
            },
            &[&msg_id, &body.attachments],
        )
        .await?;
    }

    let msg = {
        let row = t
            .query_one_cached_typed(|| super::get_one::get_one_without_perms(), &[&room_id, &msg_id])
            .await?;

        super::get_one::parse_msg(&state, &row)?
    };

    t.commit().await?;

    Ok(msg)
}
