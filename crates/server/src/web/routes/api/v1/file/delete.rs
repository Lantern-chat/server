use ftl::*;

use db::{schema::file::File, Snowflake};

pub async fn delete(route: Route<crate::ServerState>, file: File) -> Response {}
