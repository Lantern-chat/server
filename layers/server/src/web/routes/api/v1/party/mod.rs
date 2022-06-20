use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::Error;

//pub mod delete;
pub mod get;
//pub mod patch;
pub mod post;

pub mod invites;
pub mod members;
pub mod rooms;

#[rustfmt::skip]
pub async fn party(mut route: Route<crate::ServerState>) -> ApiResponse {
    let auth = crate::web::auth::authorize(&route).await?;

    match route.next().method_segment() {
        // POST /api/v1/party
        (&Method::POST, End) => post::post(route, auth).await,

        // ANY /api/v1/party/1234
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(party_id)) => match route.next().method_segment() {
                // GET /api/v1/party/1234
                (&Method::GET, End) => get::get(route, auth, party_id).await,

                // PATCH /api/v1/party/1234
                //(&Method::PATCH, End) => patch::patch(route, auth, party_id).await,

                // DELETE /api/v1/party/1234
                //(&Method::DELETE, End) => {
                //    delete::delete(route, auth, party_id).await
                //}

                // ANY /api/v1/party/1234/rooms
                (_, Exact("rooms")) => rooms::party_rooms(route, auth, party_id).await,

                // GET /api/v1/party/1234/members
                (&Method::GET, Exact("members")) => members::get_members(route, auth, party_id).await,

                _ => Err(Error::NotFound),
            },
            _ => Err(Error::BadRequest),
        },

        _ => Err(Error::NotFound),
    }
}
