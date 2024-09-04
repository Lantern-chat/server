use sdk::models::UserFlags;

use crate::prelude::*;

pub async fn ban_user(state: ServerState, user_id: UserId) -> Result<(), Error> {
    let mut db = state.db.write.get().await?;

    let t = db.transaction().await?;

    let do_ban_user = t.execute2(schema::sql! {
        UPDATE Users SET (Flags) = (Users.Flags | const {UserFlags::BANNED.bits()})
        WHERE Users.Id = #{&user_id as Users::Id}
    });

    let clear_sessions = t.execute2(schema::sql! {
        DELETE FROM Sessions WHERE Sessions.UserId = #{&user_id as Users::Id}
    });

    // TODO: Setup task to soft-delete user after 30 days or so.

    tokio::try_join!(do_ban_user, clear_sessions)?;

    t.commit().await?;

    Ok(())
}
