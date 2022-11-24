use ftl::*;
use futures::FutureExt;
use schema::Snowflake;

use crate::ServerState;

use super::ApiResponse;
use crate::{Authorization, Error};

pub mod get;

pub async fn threads(
    mut route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> ApiResponse {
    match route.next().method_segment() {
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(thread_id)) => match route.method() {
                &Method::GET => get::get(route, auth, room_id, thread_id).boxed().await,
                _ => Err(Error::MethodNotAllowed),
            },
            Some(Err(_)) => Err(Error::BadRequest),
            _ => Err(Error::MethodNotAllowed),
        },
        _ => Err(Error::MethodNotAllowed),
    }
}
