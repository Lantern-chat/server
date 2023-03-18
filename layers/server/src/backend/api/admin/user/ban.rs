use sdk::{models::UserFlags, Snowflake};

use crate::{Authorization, Error, ServerState};

pub async fn ban_user(state: ServerState, user_id: Snowflake) -> Result<(), Error> {
    let mut db = state.db.write.get().await?;

    let t = db.transaction().await?;

    let do_ban_user = async {
        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::update()
                    .table::<Users>()
                    .and_where(Users::Id.equals(Var::of(Users::Id)))
                    .set(Users::Flags, Users::Flags.bitor(UserFlags::BANNED.bits().lit()))
            },
            &[&user_id],
        )
        .await
    };

    let clear_sessions = async {
        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::delete()
                    .from::<Sessions>()
                    .and_where(Sessions::UserId.equals(Var::of(Users::Id)))
            },
            &[&user_id],
        )
        .await
    };

    // TODO: Setup task to soft-delete user after 30 days or so.

    tokio::try_join!(do_ban_user, clear_sessions)?;

    t.commit().await?;

    Ok(())
}
