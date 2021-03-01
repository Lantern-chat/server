use http::StatusCode;

use crate::{
    db::{schema::Message, Snowflake},
    server::{ftl::*, routes::api::auth::Authorization},
};

pub async fn get_one(
    mut route: Route,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> impl Reply {
    match Message::find(msg_id, &route.state.db).await {
        Ok(Some(ref msg)) => reply::json(msg).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            log::error!("Error getting message: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
