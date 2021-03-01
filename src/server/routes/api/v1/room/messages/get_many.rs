use http::StatusCode;

use crate::{
    db::{
        schema::{
            msg::{Message, MessageSearch},
            Room,
        },
        Snowflake,
    },
    server::{ftl::*, routes::api::auth::Authorization},
};

#[derive(Deserialize)]
pub struct GetManyMessagesForm {
    #[serde(flatten)]
    query: MessageSearch,

    limit: u8,
}

pub async fn get_many(mut route: Route, auth: Authorization, room_id: Snowflake) -> impl Reply {}
