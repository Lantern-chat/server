use ftl::*;

use schema::Snowflake;

use crate::web::routes::api::ApiError;
use crate::{ctrl::auth::Authorization, ServerState};

use crate::ctrl::room::messages::edit::{edit_message, EditMessageForm};

pub async fn patch(
    mut route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> Response {
    let form = match body::any::<EditMessageForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match edit_message(route.state, auth, room_id, msg_id, form).await {
        Ok(msg) => match msg {
            Some(ref msg) => reply::json(msg).into_response(),
            None => StatusCode::OK.into_response(),
        },
        Err(e) => ApiError::err(e).into_response(),
    }
}
