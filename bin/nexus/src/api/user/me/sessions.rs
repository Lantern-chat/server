use crate::prelude::*;
use sdk::models::AnonymousSession;

pub async fn list_sessions(
    state: ServerState,
    auth: Authorization,
) -> Result<impl Stream<Item = Result<AnonymousSession, Error>>, Error> {
    let db = state.db.read.get().await?;

    let sessions = db
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
            &[auth.user_id_ref()],
        )
        .await?;

    Ok(sessions.map(|row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => Ok(AnonymousSession {
            expires: row.try_get(0)?,
        }),
    }))
}

pub async fn clear_other_sessions(state: ServerState, auth: Authorization) -> Result<u64, Error> {
    let Authorization::User { token: bytes, .. } = auth else {
        return Err(Error::Unauthorized);
    };

    let db = state.db.write.get().await?;

    let num_deleted = db
        .execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::delete()
                    .from::<Sessions>()
                    .and_where(Sessions::UserId.equals(Var::of(Users::Id)))
                    .and_where(Sessions::Token.not_equals(Var::of(Sessions::Token)))
            },
            &[auth.user_id_ref(), &&bytes[..]],
        )
        .await?;

    Ok(num_deleted)
}
