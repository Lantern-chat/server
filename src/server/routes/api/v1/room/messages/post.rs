use http::StatusCode;

use crate::{
    db::{schema::Room, ClientError, Snowflake},
    server::{ftl::*, routes::api::auth::Authorization},
};

#[derive(Debug, Deserialize)]
pub struct MessagePostForm {
    content: String,
}

pub async fn post(
    mut route: Route,
    auth: Authorization,
    party_id: Snowflake,
    room_id: Snowflake,
) -> impl Reply {
    let form = match body::any::<MessagePostForm>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    "".into_response()
}

#[derive(Debug, thiserror::Error)]
pub enum MessagePostError {
    #[error(transparent)]
    ClientError(#[from] ClientError),
}

pub async fn post_message() {}
