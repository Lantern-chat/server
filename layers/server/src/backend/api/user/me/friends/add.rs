use futures::{Stream, StreamExt};

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

use sdk::models::*;

pub async fn add_friend(state: ServerState, auth: Authorization, user_id: Snowflake) -> Result<(), Error> {
    let (user_a_id, user_b_id, flags) = if auth.user_id < user_id {
        (auth.user_id, user_id, FriendFlags::empty())
    } else {
        (user_id, auth.user_id, FriendFlags::ADDED_BY)
    };

    debug_assert!(user_a_id < user_b_id);

    use q::{Parameters, Params};

    let params = Params {
        from_id: auth.user_id,
        to_id: user_id,
        user_a_id,
        user_b_id,
        flags,
    };

    let db = state.db.write.get().await?;

    let res = db
        .execute_cached_typed(|| q::query(), &params.as_params())
        .await?;

    if res == 0 {
        return Err(Error::Unauthorized);
    }

    Ok(())
}

mod q {
    use super::FriendFlags;

    pub use schema::*;
    use sdk::models::UserFlags;
    pub use thorn::*;

    thorn::params! {
        pub struct Params {
            pub from_id: Snowflake = Friends::UserAId,
            pub to_id: Snowflake = Friends::UserBId,
            pub user_a_id: Snowflake = Friends::UserAId,
            pub user_b_id: Snowflake = Friends::UserBId,
            pub flags: FriendFlags = Friends::Flags,
        }
    }

    pub fn query() -> impl AnyQuery {
        Query::insert()
            .into::<Friends>()
            .cols(&[Friends::UserAId, Friends::UserBId, Friends::Flags])
            .query(
                Query::select()
                    .exprs([Params::user_a_id(), Params::user_b_id(), Params::flags()])
                    .from(
                        Users::left_join_table::<UserBlocks>().on(UserBlocks::UserId
                            .equals(Users::Id)
                            .and(UserBlocks::BlockId.equals(Params::from_id()))),
                    )
                    .and_where(Users::Id.equals(Params::to_id()))
                    .and_where(
                        // and where user is a regular user (no bots, staff, system, etc.)
                        Users::Flags
                            .bit_and(UserFlags::ELEVATION.bits().lit())
                            .equals(0.lit()),
                    )
                    // and where not blocked
                    .and_where(UserBlocks::BlockedAt.is_null())
                    .as_value(),
            )
            .on_conflict(
                [Friends::UserAId, Friends::UserBId],
                DoUpdate
                    .set(Friends::Flags, Friends::Flags.bit_or(1.lit()))
                    .set_default(Friends::UpdatedAt)
                    .and_where(
                        // and where not accepted
                        Friends::Flags
                            .bit_and(FriendFlags::ACCEPTED.bits().lit())
                            .equals(0.lit()),
                    )
                    .and_where(
                        // and where the confirmation is from the other user, see note below
                        Params::flags()
                            .bit_xor(Friends::Flags)
                            .bit_and(FriendFlags::ADDED_BY.bits().lit())
                            .not_equals(0.lit()),
                    ),
            )
    }
}

/*
 * NOTES: Showing the ADDED_BY flag xor trick
 * where only if the other users confirms the friend request will the xor equal 1
 *
 * (user_a_id, user_b_id, added_by_flag)
 *
 * User 1 adds 2, insert (1, 2, 0)
 * User 2 adds 1, update (1, 2, 1 ^ 0)
 *
 * User 2 adds 1, insert (1, 2, 1)
 * User 1 adds 2, update (1, 2, 0 ^ 1)
 *
 * Attempts to do multiple friend-adds
 *
 * User 1 adds 2, insert (1, 2, 0)
 * User 1 adds 2, update (1, 2, 0 ^ 0)
 *
 * User 2 adds 1, insert (1, 2, 1)
 * User 2 adds 1, update (1, 2, 1 ^ 1)
 *
 */
