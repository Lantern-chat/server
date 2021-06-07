use ftl::*;

use db::Snowflake;

use crate::{ctrl::auth::Authorization, web::routes::api::ApiError};

use crate::ctrl::room::messages::get_many::{GetManyMessagesForm, MessageSearch};

pub async fn get_many(
    mut route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> impl Reply {
    let form = match body::any::<GetManyMessagesForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match crate::ctrl::room::messages::get_many::get_many(route.state, auth, room_id, form).await {
        Ok(msg) => reply::json_stream(msg).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
