use http::StatusCode;

use crate::{
    db::{schema::Room, Snowflake},
    server::{ftl::*, routes::api::auth::Authorization},
};