use super::*;

use sdk::{
    api::commands::room::{DeleteOwnReaction, PutReaction},
    models::EmoteOrEmoji,
};

use common::emoji::EmoteOrEmojiId;

pub fn reactions(
    mut route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> RouteResult {
    match route.next().method_segment() {
        (&Method::DELETE, End) => todo!("Delete all reactions"),
        (_, Exact(_)) => match route.param::<EmoteOrEmoji>() {
            Some(Ok(emote)) => {
                let Some(emote) = route.state.emoji.resolve(emote) else {
                    return Err(Error::BadRequest);
                };

                match route.next().method_segment() {
                    (&Method::GET, End) => todo!("Get reactions"),
                    (&Method::PUT, Exact("@me")) => Ok(put_reaction(route, auth, room_id, msg_id, emote)),
                    (&Method::DELETE, Exact("@me")) => Ok(delete_reaction(route, auth, room_id, msg_id, emote)),
                    (&Method::DELETE, Exact(_)) => match route.param::<Snowflake>() {
                        Some(Ok(user_id)) => todo!("Delete user reaction"),
                        _ => Err(Error::BadRequest),
                    },
                    _ => Err(Error::NotFound),
                }
            }
            _ => Err(Error::BadRequest),
        },
        (_, End) => Err(Error::MethodNotAllowed),
    }
}

#[async_recursion] #[rustfmt::skip]
async fn put_reaction(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: EmoteOrEmojiId,
) -> ApiResult {
    let Some(emote_id) = route.state.emoji.lookup(emote) else {
        return Err(Error::BadRequest);
    };

    Ok(RawMessage::authorized(auth, PutReaction { room_id, msg_id, emote_id }))
}

#[async_recursion] #[rustfmt::skip]
async fn delete_reaction(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: EmoteOrEmojiId,
) -> ApiResult {
    let Some(emote_id) = route.state.emoji.lookup(emote) else {
        return Err(Error::BadRequest);
    };

    Ok(RawMessage::authorized(auth, DeleteOwnReaction { room_id, msg_id, emote_id }))
}
