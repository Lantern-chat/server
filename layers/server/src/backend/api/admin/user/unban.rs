use sdk::{models::UserFlags, Snowflake};

use crate::{Authorization, Error, ServerState};

pub async fn unban_user(state: ServerState, user_id: Snowflake) -> Result<(), Error> {
    let db = state.db.write.get().await?;

    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;
            let not_banned_flags = (UserFlags::all() - UserFlags::BANNED).bits();

            Query::update()
                .table::<Users>()
                .and_where(Users::Id.equals(Var::of(Users::Id)))
                .set(Users::Flags, Users::Flags.bit_and(not_banned_flags.lit()))
        },
        &[&user_id],
    )
    .await?;

    Ok(())
}
