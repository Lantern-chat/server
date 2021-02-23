use http::StatusCode;

use crate::{db::{Snowflake, schema::file::File}, server::ftl::*};

pub async fn patch(route: Route, file: File) -> impl Reply {}
