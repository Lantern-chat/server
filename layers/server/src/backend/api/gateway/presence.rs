use sdk::models::{Snowflake, UserPresence, UserPresenceFlags};

use crate::{Error, ServerState};

pub async fn set_presence(
    state: ServerState,
    user_id: Snowflake,
    conn_id: Snowflake,
    presence: UserPresence,
) -> Result<(), Error> {
    let db = state.db.write.get().await?;

    use thorn::Parameters;
    thorn::params! {
        pub struct Params {
            user_id: Snowflake = schema::UserPresence::UserId,
            conn_id: Snowflake = schema::UserPresence::ConnId,
            flags: i16 = schema::UserPresence::Flags,
            activity: Option<serde_json::Value> = schema::UserPresence::Activity,
        }
    }

    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::call(schema::set_presence::call(
                Params::user_id(),
                Params::conn_id(),
                Params::flags(),
                Params::activity(),
            ))
        },
        &Params {
            user_id,
            conn_id,
            flags: presence.flags.bits(),
            activity: None,
        }
        .as_params(),
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
