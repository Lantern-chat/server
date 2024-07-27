use crate::prelude::*;
use sdk::models::*;

// TODO: Support bulk delete?
pub async fn delete_msg(
    state: ServerState,
    auth: Authorization,
    room_id: RoomId,
    msg_id: MessageId,
) -> Result<(), Error> {
    let has_perms = match state.perm_cache.get(auth.user_id, room_id).await {
        Some(ref perms) => {
            // user cannot view room at all
            if !perms.contains(Permissions::MANAGE_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            true
        }
        _ => false,
    };

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    #[rustfmt::skip]
    let Some(row) = t.query_opt2(schema::sql! {
        tables! {
            struct Selected {
                Id: Messages::Id,
                UserId: Messages::UserId,
                RoomId: Messages::RoomId,
                ParentId: Messages::ParentId,
                EditedAt: Messages::EditedAt,
                Kind: Messages::Kind,
                Flags: Messages::Flags,
                Content: Messages::Content,
                HasPerms: Type::BOOL,
            }
            struct Archived { Id: Messages::Id, UserId: Messages::UserId }
            struct SelectedAttachments { MsgId: Attachments::MsgId }
            struct DeleteAttachments {}
            struct DeleteMessage { Id: Messages::Id }
            struct InsertMessage { Id: Messages::Id }
        };

        /*
         * 1. Get message and perms
         * 2. Insert message into deleted_messages table
         * 3. Insert attachments into deleted_attachments table
         * 4. Delete original attachments
         * 5. If the message has children, update it to remove content
         */

        WITH Selected AS (
            SELECT
                Messages.Id AS Selected.Id,
                Messages.UserId AS Selected.UserId,
                Messages.RoomId AS Selected.RoomId,
                Messages.ParentId AS Selected.ParentId,
                Messages.EditedAt AS Selected.EditedAt,
                Messages.Kind AS Selected.Kind,
                Messages.Flags AS Selected.Flags,
                Messages.Content AS Selected.Content,
                if has_perms { TRUE } else {(
                    const perms: [i64; 2] = Permissions::MANAGE_MESSAGES.to_i64();
                    const ${ assert!(perms[1] == 0); }

                    AggRoomPerms.Permissions1 & const {perms[0]} = const {perms[0]}
                    OR Messages.UserId = #{auth.user_id_ref() as Users::Id}
                )} AS Selected.HasPerms

            FROM Messages if !has_perms {
                INNER JOIN AggRoomPerms
                    ON AggRoomPerms.Id = Messages.RoomId
                   AND AggRoomPerms.UserId = #{auth.user_id_ref() as Users::Id}
            }

            WHERE Messages.Id = #{&msg_id as Messages::Id}
            AND Messages.RoomId = #{&room_id as Messages::RoomId}
        ),
        Archived AS (
            INSERT INTO DeletedMessages (Id, UserId, RoomId, ParentId, EditedAt, Kind, Flags, Content, DeletedBy) (
                SELECT
                    Selected.Id, Selected.UserId, Selected.RoomId, Selected.ParentId,
                    Selected.EditedAt, Selected.Kind, Selected.Flags, Selected.Content,
                    #{auth.user_id_ref() as Users::Id}
                FROM Selected WHERE Selected.HasPerms IS TRUE
            )
            RETURNING
                DeletedMessages.Id      AS Archived.Id,
                DeletedMessages.UserId  AS Archived.UserId
        ),
        SelectedAttachments AS (
            INSERT INTO DeletedAttachments (MsgId, FileId, Flags) (
                SELECT Attachments.MsgId, Attachments.FileId, Attachments.Flags
                FROM Archived INNER JOIN Attachments ON Attachments.MsgId = Archived.Id
            )
            RETURNING DeletedAttachments.MsgId AS SelectedAttachments.MsgId
        ),
        DeleteAttachments AS (
            DELETE FROM Attachments USING SelectedAttachments
            WHERE Attachments.MsgId = SelectedAttachments.MsgId
            RETURNING Attachments.MsgId
        ),
        DeleteMessage AS (
            DELETE FROM Messages USING Archived
            WHERE Messages.Id = Archived.Id
            // don't hard-delete messages with children
            AND NOT EXISTS (SELECT FROM Messages AS Children WHERE Children.ParentId = Messages.Id)
            RETURNING Messages.Id AS DeleteMessage.Id
        ),
        InsertMessage AS (
            // if the message had children, re-insert it back but now it won't have embeds/reactions/etc.
            INSERT INTO Messages (Id, UserId, RoomId, ParentId, EditedAt, Kind, Flags) (
                SELECT  Selected.Id, Selected.UserId, Selected.RoomId, Selected.ParentId,
                        Selected.EditedAt, Selected.Kind, (Selected.Flags | CASE WHEN Selected.UserId = #{auth.user_id_ref() as Users::Id} THEN
                            // Add REMOVED if not deleted by the author
                            {MessageFlags::DELETED.bits()} ELSE {(MessageFlags::DELETED | MessageFlags::REMOVED).bits()} END
                        )
                FROM Selected INNER JOIN Archived ON TRUE
                WHERE EXISTS(SELECT FROM Messages AS Children WHERE Children.ParentId = Selected.Id)
            )
        )
        SELECT
            Selected.HasPerms AS @HadPerms,
            (SELECT COALESCE(COUNT(*), 0)::int2 FROM DeleteAttachments) AS @DeletedAttachments,
            (SELECT COALESCE(COUNT(*), 0)::int2 FROM DeleteMessage) AS @DeletedMessages,
            (SELECT COALESCE(COUNT(*), 0)::int2 FROM InsertMessage) AS @UpdatedMessages
        FROM Selected LIMIT 1
    }).await? else {
        return Err(Error::NotFound);
    };

    if !row.had_perms()? {
        return Err(Error::Unauthorized);
    }

    let _num_attachments: i16 = row.deleted_attachments()?;
    let num_deleted: i16 = row.deleted_messages()?;
    let num_updated: i16 = row.updated_messages()?;

    if (num_deleted + num_updated) == 0 {
        return Err(Error::NotFound);
    }

    t.commit().await?;

    Ok(())
}
