use http::{Method, StatusCode};

use crate::{
    db::Snowflake,
    server::{
        ftl::*,
        routes::api::auth::{authorize, Authorization},
    },
};

pub async fn messages(mut route: Route, auth: Authorization, room_id: Snowflake) -> impl Reply {}