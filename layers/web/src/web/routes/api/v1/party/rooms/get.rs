use ftl::*;

use schema::Snowflake;

use crate::{
    ctrl::{auth::Authorization, party::rooms::get::get_rooms},
    web::routes::api::ApiError,
};

pub async fn get(route: Route<crate::ServerState>, auth: Authorization, party_id: Snowflake) -> Response {
    match get_rooms(route.state, auth, party_id).await {
        Ok(ref rooms) => reply::json(rooms).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
