use hashbrown::HashSet;

use sdk::models::*;

use crate::{prelude::*, state::permission_cache::PermMute};

use sdk::api::commands::room::EditMessageBody;

pub async fn edit_message(
    state: ServerState,
    auth: Authorization,
    room_id: RoomId,
    msg_id: MessageId,
    body: &Archived<EditMessageBody>,
) -> Result<Option<Message>, Error> {
    // fast-path for if the perm_cache does contain a value
    let perms = match state.perm_cache.get(auth.user_id(), room_id).await {
        Some(PermMute { perms, .. }) => {
            // Mostly same rules as creating messages, as they are sending new content
            if !perms.contains(Permissions::SEND_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            Some(perms)
        }
        None => None,
    };

    let trimmed_content = body.content.trim();

    // early reject if empty but not containing attachments
    if trimmed_content.is_empty() && body.attachments.is_empty() {
        // TODO: Edit a mesage to have zero anything, should it be deleted instead?
        return Err(Error::BadRequest);
    }

    let mut db = state.db.write.get().await?;

    let perms = match perms {
        Some(perm) => perm,
        None => {
            let perms = crate::api::perm::get_room_permissions(&db, auth.user_id(), room_id).await?;

            if !perms.contains(Permissions::SEND_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            perms
        }
    };

    // first read-only query is not within the transaction because without repeatable-read it doesn't matter anyway
    #[rustfmt::skip]
    let prev = db.query_opt2(schema::sql! {
        const ${ assert!(!Columns::IS_DYNAMIC); }

        tables! {
            struct AggFileIds {
                FileIds: SNOWFLAKE_ARRAY,
            }
        }

        SELECT
            Messages.UserId     AS @UserId,
            Messages.Flags      AS @Flags,
            Messages.Content    AS @Content,
            AggFileIds.FileIds  AS @FileIds

        // TODO: Check if this can be optimized to not use a LATERAL
        FROM Messages LEFT JOIN LATERAL (
            SELECT ARRAY_AGG(Attachments.FileId) AS AggFileIds.FileIds
            FROM Attachments
            WHERE Attachments.MsgId = Messages.Id
        ) AS AggFileIds ON TRUE

        WHERE Messages.Id = #{&msg_id as Messages::Id}
            AND Messages.RoomId = #{&room_id as Messages::RoomId}
            AND NOT Messages.Flags & const {MessageFlags::DELETED.bits()}
    }).await?;

    let Some(row) = prev else {
        return Err(Error::NotFound);
    };

    let author_id: UserId = row.user_id()?;

    if author_id != auth.user_id() {
        return Err(Error::Unauthorized);
    }

    let _prev_flags = MessageFlags::from_bits_truncate_public(row.flags()?);
    let prev_content: Option<&str> = row.content()?;
    let prev_files: Option<Vec<FileId>> = row.file_ids()?;

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

    // edits cannot perform actions, but are subject to replacements
    let modified_content = match super::create::slash::process_slash(&trimmed_content, false) {
        Ok(Some(content)) => content,
        Ok(None) => return Ok(None),
        Err(e) => return Err(e),
    };

    if !modified_content.is_empty() && perms.contains(Permissions::EMBED_LINKS) {
        // TODO: Reprocess embeds
    }

    // this must go above the query futures creation to span their entire lexical lifetime
    let t = db.transaction().await?;

    // queue up queries to be pipelined
    use futures::future::{ok, Either};
    let mut add_attachments = Either::Left(ok::<(), Error>(()));
    let mut orphan_attachments = Either::Left(ok::<(), Error>(()));
    let mut update_message = Either::Left(ok::<(), Error>(()));

    'attachments: {
        // if no attachments were added or removed, skip the attachment processing
        if prev_files.is_none() && body.attachments.is_empty() {
            break 'attachments;
        }

        // attachments may be unordered, so a Set is required
        let pre_set: HashSet<FileId> = HashSet::from_iter(prev_files.unwrap_or_default());
        let new_set: HashSet<FileId> = HashSet::from_iter(body.attachments.as_slice().iter().copied());

        // if the sets are identical, skip the attachment processing
        if pre_set == new_set {
            break 'attachments;
        }

        let added = new_set.difference(&pre_set).copied().collect::<Vec<_>>();

        // if attachments were added and the user lacks the permission to edit attachments, reject the edit
        if !added.is_empty() && !perms.contains(Permissions::EDIT_NEW_ATTACHMENT) {
            return Err(Error::Unauthorized);
        }

        let removed = pre_set.difference(&new_set).copied().collect::<Vec<_>>();

        let t = &t; // hackery, can't take ownership of t within below async move blocks, so move a reference

        if !added.is_empty() {
            add_attachments = Either::Right(async move {
                // add new attachments
                t.execute2(schema::sql! {
                    INSERT INTO Attachments (FileId, MsgId)
                    SELECT UNNEST(#{&added as SNOWFLAKE_ARRAY}), #{&msg_id as Messages::Id}
                })
                .await?;

                Ok(())
            });
        }

        if !removed.is_empty() {
            orphan_attachments = Either::Right(async move {
                // mark removed attachments as orphaned
                t.execute2(schema::sql! {
                    const ${ assert!(!Columns::IS_DYNAMIC); }

                    UPDATE Attachments SET (Flags) = (Attachments.Flags | const {flags::AttachmentFlags::ORPHANED.bits()})
                     WHERE Attachments.FileId = ANY(#{&removed as SNOWFLAKE_ARRAY})
                })
                .await?;

                Ok(())
            });
        }
    }

    // avoid reprocessing the message content if it's identical
    if prev_content.unwrap_or("") != modified_content {
        update_message = Either::Right(async {
            t.execute2(schema::sql! {
                UPDATE Messages SET (Content, EditedAt) = (NULLIF(#{&modified_content as Messages::Content}, ""), NOW())
                 WHERE Messages.Id = #{&msg_id as Messages::Id}
            }).await?;

            Ok(())
        });
    }

    tokio::try_join!(add_attachments, orphan_attachments, update_message)?;

    let msg = super::get::get_one(state, &t, msg_id).await?;

    t.commit().await?;

    Ok(Some(msg))
}
