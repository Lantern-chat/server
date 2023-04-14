use sdk::{models::UserFlags, Snowflake};

use crate::{Authorization, Error, ServerState};

pub async fn unban_user(state: ServerState, user_id: Snowflake) -> Result<(), Error> {
    let db = state.db.write.get().await?;

    db.execute2(schema::sql! {
        UPDATE Users SET (Flags) = (Users.Flags & ~{UserFlags::BANNED.bits()})
        WHERE Users.Id = #{&user_id => Users::Id}
    }?)
    .await?;

    Ok(())
}
