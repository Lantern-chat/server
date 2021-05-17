use crate::ctrl::{auth, Error};
use crate::ServerState;

pub async fn logout_user(state: ServerState, auth: auth::Authorization) -> Result<(), Error> {
    let res = state
        .db
        .write
        .execute_cached_typed(|| delete_session(), &[&auth.token.bytes()])
        .await?;

    if res == 0 {
        log::warn!(
            "Attempted to delete nonexistent session: {}, user: {}",
            auth.token.encode(),
            auth.user_id
        );
    }

    Ok(())
}

use thorn::*;

fn delete_session() -> impl AnyQuery {
    use db::schema::*;

    Query::delete()
        .from::<Sessions>()
        .and_where(Sessions::Token.equals(Var::of(Sessions::Token)))
}
