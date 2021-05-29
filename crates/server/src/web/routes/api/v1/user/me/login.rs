use ftl::*;

use crate::{
    ctrl::user::login::{login as login_user, LoginForm},
    web::routes::api::ApiError,
    ServerState,
};

pub async fn login(mut route: Route<ServerState>) -> impl Reply {
    let form = match body::any::<LoginForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match login_user(route.state, route.addr, form).await {
        Ok(ref session) => reply::json(session).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
