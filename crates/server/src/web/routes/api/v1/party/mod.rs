use ftl::*;

use db::Snowflake;

use crate::web::{auth::authorize, routes::api::ApiError};

//pub mod delete;
pub mod get;
//pub mod patch;
pub mod post;

pub mod members;
//pub mod rooms;

pub async fn party(mut route: Route<crate::ServerState>) -> impl Reply {
    let auth = match authorize(&route).await {
        Ok(auth) => auth,
        Err(e) => return ApiError::err(e).into_response(),
    };

    match route.next().method_segment() {
        // POST /api/v1/party
        //(&Method::POST, End) => post::post(route, auth).await.into_response(),

        // ANY /api/v1/party/1234
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(party_id)) => match route.next().method_segment() {
                // GET /api/v1/party/1234
                (&Method::GET, End) => get::get(route, auth, party_id).await.into_response(),

                // PATCH /api/v1/party/1234
                //(&Method::PATCH, End) => patch::patch(route, auth, party_id).await.into_response(),

                // DELETE /api/v1/party/1234
                //(&Method::DELETE, End) => {
                //    delete::delete(route, auth, party_id).await.into_response()
                //}

                // ANY /api/v1/party/1234/rooms
                //(_, Exact("rooms")) => rooms::party_rooms(route, auth, party_id)
                //    .await
                //    .into_response(),
                (&Method::GET, Exact("members")) => members::get_members(route, auth, party_id)
                    .await
                    .into_response(),

                _ => ApiError::not_found().into_response(),
            },
            _ => return StatusCode::BAD_REQUEST.into_response(),
        },

        _ => ApiError::not_found().into_response(),
    }
}
