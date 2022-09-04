use ftl::*;

use schema::Snowflake;
use sdk::models::EmoteOrEmoji;

use super::ApiResponse;
use crate::{Authorization, Error};

pub async fn reactions(
    mut route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> ApiResponse {
    match route.next().method_segment() {
        (&Method::DELETE, End) => todo!("Delete all reactions"),
        (_, Exact(_)) => match route.param::<EmoteOrEmoji>() {
            Some(Ok(e)) => {
                match e {
                    // if emoji is not in emoji state, then reject it
                    EmoteOrEmoji::Emoji { emoji } if route.state.emoji.emoji_to_id(&emoji).is_none() => {
                        return Err(Error::BadRequest);
                    }
                    _ => {}
                }

                match route.next().method_segment() {
                    (&Method::GET, End) => todo!("Get reactions"),
                    (&Method::PUT, Exact("@me")) => todo!("Put new reaction"),
                    (&Method::DELETE, Exact("@me")) => todo!("Delete own reaction"),
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
