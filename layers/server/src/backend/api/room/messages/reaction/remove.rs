use futures::FutureExt;

use crate::{
    backend::{gateway::Event, util::encrypted_asset::encrypt_snowflake_opt},
    state::emoji::EmoteOrEmojiId,
    Authorization, Error, ServerState,
};
use sdk::models::{events::UserReactionEvent, gateway::message::ServerMsg, *};

pub async fn remove_reaction(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: EmoteOrEmojiId,
) -> Result<(), Error> {
    // TODO: Merge permission check into below CTEs?
    let perms = crate::backend::api::perm::get_cached_room_permissions(&state, auth.user_id, room_id).await?;

    if !perms.contains(Permissions::READ_MESSAGE_HISTORY) {
        return Err(Error::Unauthorized);
    }

    let db = state.db.write.get().await?;

    use q::{Parameters, Params};

    let params = Params {
        user_id: auth.user_id,
        msg_id,
        room_id,
        emote_id: emote.emote(),
        emoji_id: emote.emoji(),
    };

    let params_p = &params.as_params();

    let res = match params.emoji_id.is_some() {
        true => db.query_opt_cached_typed(|| q::query(true), params_p).boxed(),
        false => db.query_opt_cached_typed(|| q::query(false), params_p).boxed(),
    };

    let Some(row) = res.await? else { return Ok(()); };

    let party_id = row.try_get(0)?;

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
            state
                .gateway
                .broadcast_event(Event::new(event, Some(room_id))?, party_id)
                .await;
        }
        None => unimplemented!(),
    }

    Ok(())
}

mod q {
    use sdk::Snowflake;

    pub use schema::*;
    pub use thorn::*;

    thorn::tables! {
        pub struct Updated {
            MsgId: Reactions::MsgId,
        }
    }

    thorn::params! {
        pub struct Params {
            pub user_id: Snowflake = Users::Id,
            pub msg_id: Snowflake = Messages::Id,
            pub emote_id: Option<Snowflake> = Emotes::Id,
            pub emoji_id: Option<i32> = Emojis::Id,
            pub room_id: Snowflake = Rooms::Id,
        }
    }

    pub fn query(emoji: bool) -> impl AnyQuery {
        let update = Query::update()
            .table::<Reactions>()
            .set(
                Reactions::UserIds,
                Builtin::array_remove((Reactions::UserIds, Params::user_id())),
            )
            .and_where(Reactions::MsgId.equals(Params::msg_id()))
            .and_where(match emoji {
                true => Reactions::EmojiId.equals(Params::emoji_id()),
                false => Reactions::EmoteId.equals(Params::emote_id()),
            })
            .and_where(match !emoji {
                true => Params::emoji_id().is_null(),
                false => Params::emote_id().is_null(),
            })
            .and_where(Params::user_id().equals(Builtin::any(Reactions::UserIds)))
            .returning(Reactions::MsgId.alias_to(Updated::MsgId));

        Query::select()
            .with(Updated::as_query(update).exclude())
            .col(Rooms::PartyId)
            .from(Rooms::inner_join_table::<Updated>().on(true.lit()))
            .and_where(Rooms::Id.equals(Params::room_id()))
    }
}
