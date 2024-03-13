use crate::prelude::*;

use common::emoji::EmoteOrEmojiId;
use sdk::models::{events::UserReactionEvent, gateway::message::ServerMsg, *};

pub async fn remove_own_reaction(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: &Archived<EmoteOrEmoji>,
) -> Result<(), Error> {
    let Some(emote) = state.emoji.resolve(simple_de(emote)) else {
        return err(CommonError::BadRequest);
    };

    let perms = state.perm_cache.get(auth.user_id(), room_id).await;

    if matches!(perms, Some(perms) if !perms.contains(Permissions::READ_MESSAGE_HISTORY)) {
        return err(CommonError::Unauthorized);
    }

    #[rustfmt::skip]
    let res = state.db.write.get().await?.query_opt2(schema::sql! {
        tables! {
            struct SelectedReaction {
                ReactionId: Reactions::Id,
                MsgId: Reactions::MsgId,
            }

            struct DeletedReactionUser {
                ReactionId: Reactions::Id,
            }
        };

        WITH SelectedReaction AS (
            SELECT
                Reactions.Id AS SelectedReaction.ReactionId,
                Reactions.MsgId AS SelectedReaction.MsgId
            FROM Reactions

            if perms.is_none() {
                INNER JOIN AggRoomPerms ON
                    AggRoomPerms.UserId = #{auth.user_id_ref() as Users::Id}
                AND AggRoomPerms.Id     = #{&room_id as Rooms::Id}
            }

            WHERE Reactions.MsgId = #{&msg_id as Messages::Id}

            // double check that the message we're unreacting to exists
            AND EXISTS (SELECT FROM LiveMessages WHERE LiveMessages.Id = Reactions.MsgId)

            AND match emote {
                EmoteOrEmojiId::Emote(ref emote_id) => { Reactions.EmoteId = #{emote_id as Reactions::EmoteId} }
                EmoteOrEmojiId::Emoji(ref emoji_id) => { Reactions.EmojiId = #{emoji_id as Reactions::EmojiId} }
            }

            if perms.is_none() {
                let read_messages = Permissions::READ_MESSAGE_HISTORY.to_i64();

                AND AggRoomPerms.Permissions1 & {read_messages[0]} = {read_messages[0]}
                AND AggRoomPerms.Permissions2 & {read_messages[1]} = {read_messages[1]}
            }
        ), DeletedReactionUser AS (
            DELETE FROM ReactionUsers USING SelectedReaction
            WHERE ReactionUsers.ReactionId = SelectedReaction.ReactionId
            AND ReactionUsers.UserId = #{auth.user_id_ref() as Users::Id}
            RETURNING ReactionUsers.ReactionId AS DeletedReactionUser.ReactionId
        )
        SELECT
            Rooms.PartyId AS @PartyId
        FROM SelectedReaction
            INNER JOIN DeletedReactionUser ON DeletedReactionUser.ReactionId = SelectedReaction.ReactionId
            INNER JOIN Rooms ON Rooms.Id = #{&room_id as Rooms::Id}
    }).await?;

    let Some(row) = res else { return Ok(()) };

    let party_id = row.party_id()?;

    let emote = match state.emoji.lookup(emote) {
        Some(emote) => emote,
        None => {
            log::error!("Error lookup up likely valid emote/emoji: {:?}", emote);
            return Ok(());
        }
    };

    #[rustfmt::skip]
    state.gateway.events.send_simple(&ServerEvent::party(party_id, Some(room_id), ServerMsg::new_message_reaction_remove(UserReactionEvent {
        emote,
        msg_id,
        room_id,
        party_id,
        user_id: auth.user_id(),
        member: None,
    }))).await;

    Ok(())
}
