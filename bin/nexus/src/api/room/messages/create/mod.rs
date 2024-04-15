use futures::FutureExt;

use crate::{prelude::*, state::permission_cache::PermMute};

use sdk::models::*;

pub mod embed;
pub mod slash;
pub mod verify;

use sdk::api::commands::room::CreateMessageBody;

/// Returns an `Option<Message>` because slash-commands may not actually create a message
pub async fn create_message(
    state: ServerState,
    auth: Authorization,
    room_id: RoomId,
    body: &Archived<CreateMessageBody>,
) -> Result<Option<Message>, Error> {
    // fast-path for if the perm_cache does contain a value, otherwise defer until content is checked
    let perms = match state.perm_cache.get(auth.user_id(), room_id).await {
        Some(PermMute { perms, .. }) => {
            if !perms.contains(Permissions::SEND_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            Some(perms)
        }
        None => None,
    };

    let trimmed_content = body.content.as_str().trim();

    // if empty but not containing attachments
    if trimmed_content.is_empty() && body.attachments.is_empty() && body.embeds.is_empty() {
        return Err(Error::BadRequest);
    }

    // do full trimming
    let Some(trimmed_content) = ({
        let config = state.config();
        md_utils::trim_message(
            trimmed_content,
            Some(md_utils::TrimLimits {
                len: config.shared.message_length.clone(),
                max_newlines: config.shared.max_newlines as usize,
            }),
        )
    }) else {
        return Err(Error::BadRequest);
    };

    // since acquiring the database connection may be expensive,
    // defer it until we need it, such as if the permissions cache didn't have a value
    let mut maybe_db = None;

    let perms = match perms {
        Some(perm) => perm,
        None => {
            let db = state.db.write.get().await?;

            let perms = crate::api::perm::get_room_permissions(&db, auth.user_id(), room_id).await?;

            if !perms.contains(Permissions::SEND_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            maybe_db = Some(db);

            perms
        }
    };

    // check this before acquiring database connection
    if !body.attachments.is_empty() && !perms.contains(Permissions::ATTACH_FILES) {
        return Err(Error::Unauthorized);
    }

    // modify content before inserting it into the database
    let modified_content =
        match slash::process_slash(&trimmed_content, perms.contains(Permissions::USE_SLASH_COMMANDS)) {
            Ok(Some(content)) => content,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };

    let spans = md_utils::scan_markdown(&modified_content);

    // TODO: Set flags
    let flags = MessageFlags::empty();

    let msg_id = state.sf.gen();

    // if we avoided getting a database connection until now, do it now
    let mut db = match maybe_db {
        Some(db) => db,
        None => state.db.write.get().await?,
    };

    // TODO: Determine if repeatable-read is needed?
    let t = db.transaction().await?;

    // NOTE: This can potentially modify the content, hence why it takes ownership.
    // Do not assume spans are valid after this call
    let modified_content = verify::verify(&t, &state, auth, room_id, perms, modified_content).await?;

    let msg = insert_message(t, state.clone(), auth, room_id, msg_id, body, &modified_content, flags)
        .boxed()
        .await?;

    // message has been inserted, so fire off the embed processing
    if !spans.is_empty() && perms.contains(Permissions::EMBED_LINKS) {
        embed::process_embeds(state, msg_id, &modified_content, &spans);
    }

    Ok(Some(msg))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_message(
    t: db::pool::Transaction<'_>,
    state: ServerState,
    auth: Authorization,
    room_id: RoomId,
    msg_id: MessageId,
    body: &Archived<CreateMessageBody>,
    content: &str,
    flags: MessageFlags,
) -> Result<Message, Error> {
    let flags = flags.bits();

    // TODO: Threads
    #[rustfmt::skip]
    let res = t.execute2(schema::sql! {
        INSERT INTO Messages (Id, UserId, RoomId, Flags, Content) VALUES (
            #{&msg_id as Messages::Id},
            #{auth.user_id_ref() as Messages::UserId},
            #{&room_id as Messages::RoomId},
            #{&flags as Messages::Flags},
            if content.is_empty() { NULL } else { #{&content as Messages::Content} }
        )
    }).await?;

    if res != 1 {
        t.rollback().await?;
        return Err(Error::InternalErrorStatic("Unable to insert message"));
    }

    if !body.attachments.is_empty() {
        #[rustfmt::skip]
        let res = t.execute2(schema::sql! {
            INSERT INTO Attachments (MsgId, FileId) (
                SELECT #{&msg_id as Messages::Id}, UNNEST(#{&body.attachments as SNOWFLAKE_ARRAY})
            )
        })
        .await?;

        if res != body.attachments.len() as u64 {
            t.rollback().await?;
            return Err(Error::InternalErrorStatic("Unable to insert attachments"));
        }
    }

    let msg = super::get::get_one(state, &t, msg_id).await?;

    t.commit().await?;

    Ok(msg)
}
