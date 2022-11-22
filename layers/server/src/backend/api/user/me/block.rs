use sdk::models::*;
use thorn::pg::Json;

use crate::{Authorization, Error, ServerState};

pub async fn block_user(state: ServerState, auth: Authorization, user_id: Snowflake) -> Result<(), Error> {
    let mut db = state.db.write.get().await?;

    let t = db.transaction().await?;

    let do_block_user = async {
        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                let user_id_var = Var::at(UserBlocks::UserId, 1);
                let block_id_var = Var::at(UserBlocks::BlockId, 2);

                Query::insert()
                    .into::<UserBlocks>()
                    .cols(&[UserBlocks::UserId, UserBlocks::BlockId])
                    .query(
                        Query::select()
                            .from_table::<Users>()
                            .exprs([user_id_var.clone(), block_id_var.clone()])
                            .and_where(Users::Id.equals(block_id_var))
                            // can only block regular users
                            .and_where(
                                Users::Flags
                                    .bit_and(UserFlags::ELEVATION.bits().lit())
                                    .equals(0.lit()),
                            )
                            .and_where(Users::DeletedAt.is_null())
                            .as_value(),
                    )
            },
            &[&auth.user_id, &user_id],
        )
        .await
    };

    let unfriend_user = async {
        let (user_a_id, user_b_id) =
            if auth.user_id < user_id { (auth.user_id, user_id) } else { (user_id, auth.user_id) };

        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::delete()
                    .from::<Friends>()
                    .and_where(Friends::UserAId.equals(Var::of(Friends::UserAId)))
                    .and_where(Friends::UserBId.equals(Var::of(Friends::UserBId)))
            },
            &[&user_a_id, &user_b_id],
        )
        .await
    };

    let (could_block, _) = tokio::try_join!(do_block_user, unfriend_user)?;

    if could_block == 0 {
        return Err(Error::Unauthorized);
    }

    Ok(())
}
