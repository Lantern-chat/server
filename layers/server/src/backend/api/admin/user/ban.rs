use sdk::{models::UserFlags, Snowflake};

use crate::{Authorization, Error, ServerState};

pub async fn ban_user(state: ServerState, user_id: Snowflake) -> Result<(), Error> {
    let mut db = state.db.write.get().await?;

    let t = db.transaction().await?;

    let do_ban_user = t.execute2(thorn::sql! {
        use schema::*;
        UPDATE Users SET (Flags) = (Users.Flags | {UserFlags::BANNED.bits()})
        WHERE Users.Id = #{&user_id => Users::Id}
    }?);

    let clear_sessions = t.execute2(thorn::sql! {
        use schema::*;
        DELETE FROM Sessions WHERE Sessions.UserId = #{&user_id => Users::Id}
    }?);

    // TODO: Setup task to soft-delete user after 30 days or so.

    tokio::try_join!(do_ban_user, clear_sessions)?;

    t.commit().await?;

    Ok(())
}
