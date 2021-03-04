use http::{Method, StatusCode};

use crate::{
    db::Snowflake,
    server::{ftl::*, routes::api::auth::authorize},
};
