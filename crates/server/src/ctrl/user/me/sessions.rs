use futures::stream::{Stream, StreamExt};

use crate::{
    ctrl::{auth::Authorization, Error},
    ServerState,
};

use models::AnonymousSession;

pub async fn list_sessions(
    state: ServerState,
    auth: Authorization,
) -> Result<impl Stream<Item = Result<AnonymousSession, Error>>, Error> {
    let sessions = state
        .read_db()
        .await
        .query_stream_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .col(Sessions::Expires)
                    .from_table::<Sessions>()
                    .and_where(Sessions::UserId.equals(Var::of(Sessions::UserId)))
                    .order_by(Sessions::Expires.ascending())
            },
            &[&auth.user_id],
        )
        .await?;

    Ok(sessions.map(|row| {
        Ok(AnonymousSession {
            expires: crate::util::time::format_naivedatetime(row?.try_get::<_, chrono::NaiveDateTime>(0)?),
        })
    }))
}
