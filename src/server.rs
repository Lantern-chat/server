use std::sync::Arc;

use crate::state::ServerState;

pub async fn start_server(state: Arc<ServerState>) {
    warp::serve(crate::routes::routes(state))
        .run(([127, 0, 0, 1], 3030))
        .await
}
