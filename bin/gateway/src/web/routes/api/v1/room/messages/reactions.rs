use super::*;

use sdk::{
    api::commands::room::{DeleteOwnReaction, PutReaction},
    models::EmoteOrEmoji,
};

use common::emoji::EmoteOrEmojiId;

pub fn reactions(
    mut route: Route<ServerState>,
    auth: Authorization,
    room_id: RoomId,
    msg_id: MessageId,
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
                    (&Method::DELETE, Exact(_)) => match route.param::<UserId>() {
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

#[async_recursion]
async fn put_reaction(
    route: Route<ServerState>,
    _auth: Authorization,
    room_id: RoomId,
    msg_id: MessageId,
    emote: EmoteOrEmojiId,
) -> ApiResult {
    let Some(emote_id) = route.state.emoji.lookup(emote) else {
        return Err(Error::BadRequest);
    };

    Ok(Procedure::from(PutReaction {
        room_id,
        msg_id,
        emote_id,
    }))
}

#[async_recursion]
async fn delete_reaction(
    route: Route<ServerState>,
    _auth: Authorization,
    room_id: RoomId,
    msg_id: MessageId,
    emote: EmoteOrEmojiId,
) -> ApiResult {
    let Some(emote_id) = route.state.emoji.lookup(emote) else {
        return Err(Error::BadRequest);
    };

    Ok(Procedure::from(DeleteOwnReaction {
        room_id,
        msg_id,
        emote_id,
    }))
}
