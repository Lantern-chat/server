use schema::auth::RawAuthToken;

use crate::ctrl::{auth, Error};
use crate::ServerState;

pub async fn logout_user(state: ServerState, auth: auth::Authorization) -> Result<(), Error> {
    let bytes = match auth.token {
        RawAuthToken::Bearer(ref bytes) => &bytes[..],
        _ => return Err(Error::BadRequest),
    };

    let db = state.db.write.get().await?;

    let res = db
        .execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::delete()
                    .from::<Sessions>()
                    .and_where(Sessions::Token.equals(Var::of(Sessions::Token)))
            },
            &[&bytes],
        )
        .await?;

    if res == 0 {
        log::warn!(
            "Attempted to delete nonexistent session: {}, user: {}",
            auth.token,
            auth.user_id
        );
    }

    Ok(())
}
