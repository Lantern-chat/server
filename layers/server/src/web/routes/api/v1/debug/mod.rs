use ftl::*;

use crate::{Error, ServerState};

pub async fn debug(mut route: Route<ServerState>) -> Result<Response, Error> {
    match route.next().segment() {
        Exact("exception") => test_exception(route.state).await,
        _ => Ok(().into_response()),
    }
}

#[async_recursion]
async fn test_exception(state: ServerState) -> Result<Response, Error> {
    let db = state.db.read.get().await.unwrap();

    match db.execute("CALL lantern.do_thing()", &[]).await {
        Ok(_) => {}
        Err(e) => {
            if let Some(db) = e.as_db_error() {
                log::debug!("{:?}", db);
            }
        }
    }

    Ok(().into_response())
}
