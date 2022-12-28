use super::*;

pub fn debug(mut route: Route<ServerState>) -> RouteResult {
    match route.next().segment() {
        Exact("exception") => Ok(test_exception(route.state)),
        _ => Err(Error::NotFound),
    }
}

#[async_recursion]
async fn test_exception(state: ServerState) -> WebResult {
    let db = state.db.read.get().await.unwrap();

    match db.execute("CALL lantern.do_thing()", &[]).await {
        Ok(_) => {}
        Err(e) => {
            if let Some(db) = e.as_db_error() {
                log::debug!("{:?}", db);
            }
        }
    }

    Ok(().into())
}
