use ftl::*;

use schema::Snowflake;

use crate::{ctrl::auth::Authorization, web::routes::api::ApiError};

pub async fn get_many(route: Route<crate::ServerState>, auth: Authorization, room_id: Snowflake) -> Response {
    let form = match route.query() {
        None => Default::default(),
        Some(Ok(form)) => form,
        Some(Err(e)) => return ApiError::err(e.into()).into_response(),
    };

    match crate::ctrl::room::messages::get_many::get_many(route.state, auth, room_id, form).await {
        Ok(msg) => reply::json::array_stream(msg).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
