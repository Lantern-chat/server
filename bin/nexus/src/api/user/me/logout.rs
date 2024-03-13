use crate::prelude::*;

pub async fn logout_user(state: ServerState, auth: Authorization) -> Result<(), Error> {
    let Authorization::User { token, user_id, .. } = auth else {
        return Err(Error::BadRequest);
    };

    let db = state.db.write.get().await?;
    let bytes = token.as_slice();

    #[rustfmt::skip]
    let res = db.execute2(schema::sql! {
        DELETE FROM Sessions WHERE Sessions.Token = #{&bytes as Sessions::Token}
    }).await?;

    if res == 0 {
        log::warn!("Attempted to delete nonexistent session: {token:?}, user: {user_id}");
    }

    Ok(())
}
