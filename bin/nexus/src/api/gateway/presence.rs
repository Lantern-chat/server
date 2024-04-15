use sdk::models::UserPresence;

use crate::prelude::*;

pub async fn set_presence(
    state: ServerState,
    user_id: UserId,
    conn_id: ConnectionId,
    presence: UserPresence,
) -> Result<(), Error> {
    let flags = presence.flags.bits();

    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        CALL .set_presence(
            #{&user_id as UserPresence::UserId},
            #{&conn_id as UserPresence::ConnId},
            #{&flags   as UserPresence::Flags},
            NULL // #{&activity as UserPresence::Activity}
        )
    }).await?;

    Ok(())
}

pub async fn clear_presence(state: ServerState, user_id: UserId, conn_id: ConnectionId) -> Result<(), Error> {
    #[rustfmt::skip]
    state.db.write.get().await?.execute2(schema::sql! {
        DELETE FROM UserPresence
        WHERE UserPresence.UserId = #{&user_id as UserPresence::UserId}
          AND UserPresence.ConnId = #{&conn_id as UserPresence::ConnId}
    }).await?;

    Ok(())
}
