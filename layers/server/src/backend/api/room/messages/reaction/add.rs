use crate::{state::emoji::EmoteOrEmojiId, Authorization, Error, ServerState};
use futures::FutureExt;
use sdk::models::{EmoteOrEmoji, RoomPermissions, Snowflake};

pub async fn add_reaction(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: EmoteOrEmojiId,
) -> Result<(), Error> {
    let permissions =
        crate::backend::api::perm::get_cached_room_permissions(&state, auth.user_id, room_id).await?;

    if !permissions.contains(RoomPermissions::ADD_REACTIONS) {
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

    #[rustfmt::skip]
    let res = match (
        permissions.contains(RoomPermissions::USE_EXTERNAL_EMOTES),
        params.emoji_id.is_some(),
    ) {
        (false, false) => db.execute_cached_typed(|| q::query(false, false), params_p).boxed(),
        (false, true) => db.execute_cached_typed(|| q::query(false, true), params_p).boxed(),
        (true, false) => db.execute_cached_typed(|| q::query(true, false), params_p).boxed(),
        (true, true) => db.execute_cached_typed(|| q::query(true, true), params_p).boxed(),
    };

    let res = res.await?;

    Ok(())
}

mod q {
    pub use schema::*;
    pub use thorn::*;

    use super::*;

    thorn::params! {
        pub struct Params {
            pub user_id: Snowflake = Users::Id,
            pub msg_id: Snowflake = Messages::Id,
            pub emote_id: Option<Snowflake> = Emotes::Id,
            pub emoji_id: Option<i32> = Emojis::Id,
            pub room_id: Snowflake = Rooms::Id,
        }
    }

    pub fn query(allow_external: bool, emoji: bool) -> impl AnyQuery {
        let mut values = Query::select()
            .exprs([Params::msg_id(), Params::emote_id(), Params::emoji_id()])
            .expr(Builtin::array(Params::user_id()))
            // room_id may be unused, so just toss it here, will always be true
            .and_where(Params::room_id().is_not_null());

        if !emoji {
            values = match allow_external {
                true => values
                    .from(
                        PartyMember::inner_join_table::<Emotes>()
                            .on(Emotes::PartyId.equals(PartyMember::PartyId)),
                    )
                    .and_where(PartyMember::UserId.equals(Params::user_id())),
                false => values
                    .from(Rooms::inner_join_table::<Emotes>().on(Emotes::PartyId.equals(Rooms::PartyId)))
                    .and_where(Rooms::Id.equals(Params::room_id())),
            };

            values = values.and_where(Emotes::Id.equals(Params::emote_id()));
        }

        let q = Query::insert()
            .into::<Reactions>()
            .cols(&[
                Reactions::MsgId,
                Reactions::EmoteId,
                Reactions::EmojiId,
                Reactions::UserIds,
            ])
            .query(values.as_value())
            .on_conflict(
                match emoji {
                    true => [Reactions::MsgId, Reactions::EmojiId],
                    false => [Reactions::MsgId, Reactions::EmoteId],
                },
                DoUpdate
                    .set(
                        Reactions::UserIds,
                        Builtin::array_append((Reactions::UserIds, Params::user_id())),
                    )
                    .and_where(Params::user_id().not_equals(Builtin::any((Reactions::UserIds,)))),
            );

        q
    }
}
