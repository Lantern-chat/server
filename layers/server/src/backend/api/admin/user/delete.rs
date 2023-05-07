use rand::distributions::{Alphanumeric, DistString};
use sdk::Snowflake;

use crate::{Authorization, Error, ServerState};

// NOTE: It's assumed that the calling user has permission to do this
pub async fn delete_user(state: ServerState, user_id: Snowflake) -> Result<(), Error> {
    // generate 10 alphanumeric characters for the new username
    let mut new_username = "DeletedUser ".to_owned();
    Alphanumeric.append_string(&mut rand::thread_rng(), &mut new_username, 10);

    let db = state.db.write.get().await?;

    // TODO: Typecheck this procedure
    db.execute2(schema::sql! {
        CALL .soft_delete_user(
            #{&user_id as Users::Id},
            #{&new_username as Users::Username}
        )
    })
    .await?;

    Ok(())
}
