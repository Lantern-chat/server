use ftl::*;

use schema::Snowflake;

use crate::web::routes::api::ApiError;
use crate::{ctrl::auth::Authorization, ServerState};

use crate::ctrl::room::messages::create::{create_message, CreateMessageForm};

pub async fn post(mut route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> Response {
    let form = match body::any::<CreateMessageForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match create_message(route.state, auth, room_id, form).await {
        Ok(msg) => match msg {
            Some(ref msg) => reply::json(msg).into_response(),
            None => StatusCode::OK.into_response(),
        },
        Err(e) => ApiError::err(e).into_response(),
    }
}
