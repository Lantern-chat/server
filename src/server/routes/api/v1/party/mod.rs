use http::{Method, StatusCode};

use crate::{
    db::Snowflake,
    server::{
        ftl::*,
        routes::api::auth::{authorize, Authorization},
    },
};

pub mod create;
pub mod get;

pub async fn party(mut route: Route) -> impl Reply {
    let auth = match authorize(&route).await {
        Ok(auth) => auth,
        Err(err) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    match route.next().method_segment() {
        // POST /api/v1/party
        (&Method::POST, End) => create::create(route, auth).await.into_response(),

        // ANY /api/v1/party/1234
        _ => match route.param::<Snowflake>() {
            Some(Ok(party_id)) => match route.next().method_segment() {
                // GET /api/v1/party/1234
                (&Method::GET, End) => get::get(route, auth, party_id).await.into_response(),

                _ => StatusCode::NOT_FOUND.into_response(),
            },
            _ => return StatusCode::BAD_REQUEST.into_response(),
        },
    }
}
