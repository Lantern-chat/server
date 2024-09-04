use crate::prelude::*;
use sdk::models::AnonymousSession;

pub async fn list_sessions(
    state: ServerState,
    auth: Authorization,
) -> Result<impl Stream<Item = Result<AnonymousSession, Error>>, Error> {
    let db = state.db.read.get().await?;

    #[rustfmt::skip]
    let sessions = db.query_stream2(schema::sql! {
        SELECT Sessions.Expires AS @_ FROM Sessions
        WHERE Sessions.UserId = #{auth.user_id_ref() as Users::Id}
        ORDER BY Sessions.Expires ASC
    }).await?;

    Ok(sessions.map(|row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => Ok(AnonymousSession {
            expires: row.sessions_expires()?,
        }),
    }))
}

pub async fn clear_other_sessions(state: ServerState, auth: Authorization) -> Result<u64, Error> {
    let Authorization::User { token: bytes, .. } = auth else {
        return Err(Error::Unauthorized);
    };

    let db = state.db.write.get().await?;
    let bytes = &bytes[..];

    #[rustfmt::skip]
    let num_deleted = db.execute2(schema::sql! {
        DELETE FROM Sessions
        WHERE Sessions.UserId = #{auth.user_id_ref() as Users::Id}
          AND Sessions.Token != #{&bytes as Sessions::Token}
    }).await?;

    Ok(num_deleted)
}
