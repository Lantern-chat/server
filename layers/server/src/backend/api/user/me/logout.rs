use schema::auth::RawAuthToken;

use crate::{Authorization, Error, ServerState};

pub async fn logout_user(state: &ServerState, auth: Authorization) -> Result<(), Error> {
    let RawAuthToken::Bearer(ref bytes) = auth.token else {
        return Err(Error::BadRequest);
    };

    let db = state.db.write.get().await?;
    let bytes = bytes.as_slice();

    #[rustfmt::skip]
    let res = db.execute2(schema::sql! {
        DELETE FROM Sessions WHERE Sessions.Token = #{&bytes as Sessions::Token}
    }).await?;

    if res == 0 {
        log::warn!(
            "Attempted to delete nonexistent session: {}, user: {}",
            auth.token,
            auth.user_id
        );
    }

    Ok(())
}
