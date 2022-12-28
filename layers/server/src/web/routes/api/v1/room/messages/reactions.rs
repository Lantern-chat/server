use super::*;

use sdk::models::EmoteOrEmoji;

use crate::state::emoji::EmoteOrEmojiId;

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
                    return Err(Error::BadRequest)
                };

                match route.next().method_segment() {
                    (&Method::GET, End) => todo!("Get reactions"),
                    (&Method::PUT, Exact("@me")) => Ok(put_reaction(route, auth, room_id, msg_id, emote)),
                    (&Method::DELETE, Exact("@me")) => Ok(delete_reaction(route, auth, room_id, msg_id, emote)),
                    (&Method::DELETE, Exact(_)) => match route.param::<Snowflake>() {
                        Some(Ok(user_id)) => todo!("Delete user reaction"),
                        _ => return Err(Error::BadRequest),
                    },
                    _ => return Err(Error::NotFound),
                }
            }
            _ => return Err(Error::BadRequest),
        },
        (_, End) => return Err(Error::MethodNotAllowed),
    }
}

#[async_recursion]
async fn put_reaction(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: EmoteOrEmojiId,
) -> WebResult {
    crate::backend::api::room::messages::reaction::add::add_reaction(route.state, auth, room_id, msg_id, emote)
        .await?;

    Ok(StatusCode::NO_CONTENT.into())
}

#[async_recursion]
async fn delete_reaction(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: EmoteOrEmojiId,
) -> WebResult {
    crate::backend::api::room::messages::reaction::remove::remove_reaction(
        route.state,
        auth,
        room_id,
        msg_id,
        emote,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT.into())
}
