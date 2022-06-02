use ftl::*;

use crate::ServerState;

pub async fn debug(mut route: Route<ServerState>) -> Response {
    match route.next().segment() {
        Exact("exception") => test_exception(route.state).await,
        _ => ().into_response(),
    }
}

async fn test_exception(state: ServerState) -> Response {
    let db = state.db.read.get().await.unwrap();

    match db.execute("CALL lantern.do_thing()", &[]).await {
        Ok(_) => {}
        Err(e) => {
            if let Some(db) = e.as_db_error() {
                log::debug!("{:?}", db);
            }
        }
    }

    ().into_response()
}
