use futures::FutureExt;

use crate::{
    backend::{gateway::Event, util::encrypted_asset::encrypt_snowflake_opt},
    state::emoji::EmoteOrEmojiId,
    Authorization, Error, ServerState,
};
use sdk::models::{events::UserReactionEvent, gateway::message::ServerMsg, *};

pub async fn remove_own_reaction(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: EmoteOrEmojiId,
) -> Result<(), Error> {
    let perms = state.perm_cache.get(auth.user_id, room_id).await;

    match perms {
        Some(perms) if !perms.contains(Permissions::READ_MESSAGE_HISTORY) => return Err(Error::Unauthorized),
        _ => {}
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
                    AggRoomPerms.UserId = #{&auth.user_id as Users::Id}
                AND AggRoomPerms.Id     = #{&room_id as Rooms::Id}
            }

            WHERE Reactions.MsgId = #{&msg_id as Messages::Id}
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
            AND ReactionUsers.UserId = #{&auth.user_id as Users::Id}
            RETURNING ReactionUsers.ReactionId AS DeletedReactionUser.ReactionId
        )
        SELECT
            Rooms.PartyId AS @PartyId
        FROM SelectedReaction
            INNER JOIN DeletedReactionUser ON DeletedReactionUser.ReactionId = SelectedReaction.ReactionId
            INNER JOIN Rooms ON Rooms.Id = #{&room_id as Rooms::Id}
    }).await?;

    let Some(row) = res else { return Ok(()); };

    let party_id = row.party_id()?;

    let emote = match state.emoji.lookup(emote) {
        Some(emote) => emote,
        None => {
            log::error!("Error lookup up likely valid emote/emoji: {:?}", emote);
            return Ok(());
        }
    };

    let event = ServerMsg::new_message_reaction_remove(UserReactionEvent {
        emote,
        msg_id,
        room_id,
        party_id,
        user_id: auth.user_id,
        member: None,
    });

    match party_id {
        Some(party_id) => {
            state.gateway.broadcast_event(Event::new(event, Some(room_id))?, party_id);
        }
        None => unimplemented!(),
    }

    Ok(())
}

// mod q {
//     use sdk::Snowflake;

//     pub use schema::*;
//     pub use thorn::*;

//     thorn::tables! {
//         pub struct Updated {
//             MsgId: Reactions::MsgId,
//         }
//     }

//     thorn::params! {
//         pub struct Params {
//             pub user_id: Snowflake = Users::Id,
//             pub msg_id: Snowflake = Messages::Id,
//             pub emote_id: Option<Snowflake> = Emotes::Id,
//             pub emoji_id: Option<i32> = Emojis::Id,
//             pub room_id: Snowflake = Rooms::Id,
//         }
//     }

//     pub fn query(emoji: bool) -> impl AnyQuery {
//         let update = Query::update()
//             .table::<Reactions>()
//             .set(
//                 Reactions::UserIds,
//                 Builtin::array_remove((Reactions::UserIds, Params::user_id())),
//             )
//             .and_where(Reactions::MsgId.equals(Params::msg_id()))
//             .and_where(match emoji {
//                 true => Reactions::EmojiId.equals(Params::emoji_id()),
//                 false => Reactions::EmoteId.equals(Params::emote_id()),
//             })
//             .and_where(match !emoji {
//                 true => Params::emoji_id().is_null(),
//                 false => Params::emote_id().is_null(),
//             })
//             .and_where(Params::user_id().equals(Builtin::any(Reactions::UserIds)))
//             .returning(Reactions::MsgId.alias_to(Updated::MsgId));

//         Query::select()
//             .with(Updated::as_query(update).exclude())
//             .col(Rooms::PartyId)
//             .from(Rooms::inner_join_table::<Updated>().on(true.lit()))
//             .and_where(Rooms::Id.equals(Params::room_id()))
//     }
// }
