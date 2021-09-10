use ftl::*;

use crate::ctrl::auth;
use crate::ctrl::user::me::prefs::update_prefs;
use crate::web::routes::api::ApiError;
use crate::ServerState;

use models::UserPreferences;

pub async fn prefs(mut route: Route<ServerState>, auth: auth::Authorization) -> Response {
    let form = match body::any::<UserPreferences, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match update_prefs(route.state, auth, form).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
