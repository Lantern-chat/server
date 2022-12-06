use rand::distributions::{Alphanumeric, DistString};
use sdk::Snowflake;

use crate::{Authorization, Error, ServerState};

// NOTE: It's assumed that the calling user has permission to do this
pub async fn delete_user(state: ServerState, user_id: Snowflake) -> Result<(), Error> {
    // generate 10 alphanumeric characters for the new username
    let mut new_username = "DeletedUser ".to_owned();
    Alphanumeric.append_string(&mut rand::thread_rng(), &mut new_username, 10);

    let db = state.db.write.get().await?;

    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::call(schema::soft_delete_user(
                Var::of(Users::Id),
                Var::of(Users::Username),
            ))
        },
        &[&user_id, &new_username],
    )
    .await?;

    Ok(())
}
