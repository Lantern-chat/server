use futures::FutureExt;
use schema::{Snowflake, SnowflakeExt};

use crate::{backend::cache::permission_cache::PermMute, Authorization, Error, ServerState};

use sdk::models::*;

pub mod embed;
pub mod slash;
pub mod trim;

use sdk::api::commands::room::CreateMessageBody;

/// Returns an `Option<Message>` because slash-commands may not actually create a message
pub async fn create_message(
    state: ServerState,
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

            let perm = crate::backend::api::perm::get_room_permissions(&db, auth.user_id, room_id).await?;

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
    state: ServerState,
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

        use schema::*;
        use thorn::*;

        params! {
            pub struct Params<'a> {
                pub msg_id: Snowflake = Messages::Id,
                pub user_id: Snowflake = Users::Id,
                pub room_id: Snowflake = Rooms::Id,
                pub parent_msg_id: Snowflake = Messages::Id,
                pub thread_id: Snowflake = Threads::Id,
                pub new_thread_flags: ThreadFlags = Threads::Flags,
                pub content: Option<&'a str> = Messages::Content,
            }
        }

        t.execute_cached_typed(
            || {
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
                                Params::thread_id(),
                                Params::parent_msg_id(),
                                Params::new_thread_flags(),
                            )))
                            .as_value(),
                    )
                    .values([
                        Params::msg_id(),
                        Params::user_id(),
                        Params::room_id(),
                        Params::content(),
                    ])
            },
            &Params {
                msg_id,
                user_id: auth.user_id,
                room_id,
                parent_msg_id,
                thread_id,
                new_thread_flags: ThreadFlags::empty(), // TODO
                content,
            }
            .as_params(),
        )
        .await?;
    } else {
        use schema::*;
        use thorn::*;

        params! {
            pub struct Params<'a> {
                msg_id: Snowflake = Messages::Id,
                user_id: Snowflake = Messages::UserId,
                room_id: Snowflake = Messages::RoomId,
                content: Option<&'a str> = Messages::Content,
            }
        }

        t.execute_cached_typed(
            || {
                Query::insert()
                    .into::<Messages>()
                    .cols(&[
                        Messages::Id,
                        Messages::UserId,
                        Messages::RoomId,
                        Messages::Content,
                    ])
                    .values([
                        Params::msg_id(),
                        Params::user_id(),
                        Params::room_id(),
                        Params::content(),
                    ])
            },
            &Params {
                msg_id,
                user_id: auth.user_id,
                room_id,
                content,
            }
            .as_params(),
        )
        .await?;
    }

    if !body.attachments.is_empty() {
        use schema::*;
        use thorn::*;

        params! {
            pub struct Params<'a> {
                msg_id: Snowflake = Messages::Id,
                attachments: &'a [Snowflake] = SNOWFLAKE_ARRAY,
            }
        }

        t.execute_cached_typed(
            || {
                tables! {
                    struct AggIds {
                        Id: Files::Id,
                    }
                }

                Query::with()
                    .with(
                        AggIds::as_query(
                            Query::select().expr(
                                Call::custom("UNNEST")
                                    .arg(Params::attachments())
                                    .alias_to(AggIds::Id),
                            ),
                        )
                        .exclude(),
                    )
                    .insert()
                    .into::<Attachments>()
                    .cols(&[Attachments::FileId, Attachments::MessageId])
                    .query(
                        Query::select()
                            .col(AggIds::Id)
                            .expr(Params::msg_id())
                            .from_table::<AggIds>()
                            .as_value(),
                    )
            },
            &Params {
                msg_id,
                attachments: &body.attachments,
            }
            .as_params(),
        )
        .await?;
    }

    let msg = super::get::get_one_transactional(state, msg_id, room_id, &t).await?;

    t.commit().await?;

    Ok(msg)
}
