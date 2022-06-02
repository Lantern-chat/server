use ftl::*;

use crate::{ctrl::user::me::login::login as login_user, web::routes::api::ApiError, ServerState};

pub async fn login(mut route: Route<ServerState>) -> Response {
    let form = match body::any(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match login_user(route.state, route.real_addr, form).await {
        Ok(ref session) => reply::json(session)
            .with_status(StatusCode::CREATED)
            .into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
