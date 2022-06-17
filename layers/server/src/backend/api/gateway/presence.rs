use sdk::models::{Snowflake, UserPresence};

use crate::{Error, ServerState};

pub async fn set_presence(
    state: ServerState,
    user_id: Snowflake,
    conn_id: Snowflake,
    presence: UserPresence,
) -> Result<(), Error> {
    let db = state.db.write.get().await?;

    let activity: Option<serde_json::Value> = None;

    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::call(Call::custom("lantern.set_presence").args((
                Var::of(UserPresence::UserId),
                Var::of(UserPresence::ConnId),
                Var::of(UserPresence::Flags),
                Var::of(UserPresence::Activity),
            )))
        },
        &[&user_id, &conn_id, &presence.flags.bits(), &activity],
    )
    .await?;

    Ok(())
}

pub async fn clear_presence(state: ServerState, conn_id: Snowflake) -> Result<(), Error> {
    let db = state.db.write.get().await?;

    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::delete()
                .from::<UserPresence>()
                .and_where(UserPresence::ConnId.equals(Var::of(UserPresence::ConnId)))
        },
        &[&conn_id],
    )
    .await?;

    Ok(())
}
