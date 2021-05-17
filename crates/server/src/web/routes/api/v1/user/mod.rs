use ftl::*;

use crate::web::{auth::authorize, routes::api::ApiError};

//pub mod check;
pub mod register;

pub mod me;

pub async fn user(mut route: Route<crate::ServerState>) -> impl Reply {
    match route.next().method_segment() {
        // POST /api/v1/user
        (&Method::POST, End) => register::register(route).await.into_response(),

        // ANY /api/v1/user/@me
        (_, Exact("@me")) => me::me(route).await.into_response(),

        // ANY /api/v1/user/1234
        (_, Exact(segment)) => match segment.parse::<db::Snowflake>() {
            Err(_) => StatusCode::BAD_REQUEST.into_response(),
            Ok(_user_id) => match authorize(&route).await {
                Err(e) => ApiError::err(e).into_response(),
                Ok(_auth) => "user stuff".into_response(),
            },
        },
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
