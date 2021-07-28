use ftl::*;
use models::Snowflake;

use crate::{web::auth::Authorization, ServerState};

pub async fn head(route: Route<ServerState>, auth: Authorization, file_id: Snowflake) -> Response {
    //let db = route.state.db.read.get().await?;

    ().into_response()
}
