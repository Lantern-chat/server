use std::{sync::Arc, time::SystemTime};

use db::{ClientError, Snowflake, SnowflakeExt};

use ftl::*;

use crate::{
    ctrl::{
        user::register::{register as register_user, RegisterForm},
        Error,
    },
    web::routes::api::ApiError,
    ServerState,
};

pub async fn register(mut route: Route<ServerState>) -> impl Reply {
    let form = match body::any::<RegisterForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match register_user(route.state, form).await {
        Ok(ref session) => reply::json(session).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
