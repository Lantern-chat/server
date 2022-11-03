use ftl::*;

use futures::FutureExt;
use schema::Snowflake;
use sdk::models::EmoteOrEmoji;

use super::ApiResponse;
use crate::{state::emoji::EmoteOrEmojiId, Authorization, Error, ServerState};

pub async fn reactions(
    mut route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> ApiResponse {
    match route.next().method_segment() {
        (&Method::DELETE, End) => todo!("Delete all reactions"),
        (_, Exact(_)) => match route.param::<EmoteOrEmoji>() {
            Some(Ok(emote)) => {
                let Some(emote) = route.state.emoji.resolve(emote) else { return Err(Error::BadRequest) };

                match route.next().method_segment() {
                    (&Method::GET, End) => todo!("Get reactions"),
                    (&Method::PUT, Exact("@me")) => {
                        put_reaction(route, auth, room_id, msg_id, emote).boxed().await
                    }
                    (&Method::DELETE, Exact("@me")) => {
                        delete_reaction(route, auth, room_id, msg_id, emote).boxed().await
                    }
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

async fn put_reaction(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: EmoteOrEmojiId,
) -> ApiResponse {
    crate::backend::api::room::messages::reaction::add::add_reaction(
        route.state,
        auth,
        room_id,
        msg_id,
        emote,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn delete_reaction(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: EmoteOrEmojiId,
) -> ApiResponse {
    crate::backend::api::room::messages::reaction::remove::remove_reaction(
        route.state,
        auth,
        room_id,
        msg_id,
        emote,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}
