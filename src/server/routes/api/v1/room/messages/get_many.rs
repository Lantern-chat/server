use http::StatusCode;

use crate::{
    db::{
        schema::{Message, MessageSearch, Room},
        Snowflake,
    },
    server::{ftl::*, routes::api::auth::Authorization},
};

#[derive(Deserialize)]
pub struct GetManyMessagesForm {
    #[serde(flatten)]
    query: MessageSearch,

    #[serde(default = "default_limit")]
    limit: u8,
}

#[rustfmt::skip]
const fn default_limit() -> u8 { 50 }

pub async fn get_many(mut route: Route, auth: Authorization, room_id: Snowflake) -> impl Reply {
    let form = match body::any::<GetManyMessagesForm>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match Message::search(&route.state.db, room_id, form.limit, form.query).await {
        Ok(msg) => reply::json_stream(msg).into_response(),
        Err(e) => {
            log::error!("Error getting many messsages: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
