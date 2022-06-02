use ftl::*;

use crate::{ctrl::auth::Authorization, web::routes::api::ApiError, ServerState};

use crate::ctrl::party::create::{create_party, PartyCreateForm};

pub async fn post(mut route: Route<ServerState>, auth: Authorization) -> Response {
    let form = match body::any::<PartyCreateForm, _>(&mut route).await {
        Ok(form) => form,
        Err(_) => return ApiError::bad_request().into_response(),
    };

    match create_party(route.state, auth, form).await {
        Ok(ref party) => reply::json(party).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
