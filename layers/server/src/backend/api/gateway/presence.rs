use sdk::models::{Snowflake, UserPresence, UserPresenceFlags};

use crate::{Error, ServerState};

pub async fn set_presence(
    state: ServerState,
    user_id: Snowflake,
    conn_id: Snowflake,
    presence: UserPresence,
) -> Result<(), Error> {
    let flags = presence.flags.bits();

    #[rustfmt::skip]
    let db = state.db.write.get().await?.execute2(thorn::sql! {
        use schema::*;
        CALL .set_presence(
            #{&user_id => UserPresence::UserId},
            #{&conn_id => UserPresence::ConnId},
            #{&flags   => UserPresence::Flags},
            NULL // #{&activity => UserPresence::Activity}
        )
    }?).await?;

    Ok(())
}

pub async fn clear_presence(state: ServerState, conn_id: Snowflake) -> Result<(), Error> {
    #[rustfmt::skip]
    state.db.write.get().await?.execute2(thorn::sql! {
        use schema::*;
        DELETE FROM UserPresence WHERE UserPresence.ConnId = #{&conn_id => UserPresence::ConnId}
    }?).await?;

    Ok(())
}
