use futures::{Stream, StreamExt};
use schema::SnowflakeExt;

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, prelude::*};

use sdk::models::*;

pub async fn add_friend(state: ServerState, auth: Authorization, user_id: UserId) -> Result<(), Error> {
    let (user_a_id, user_b_id) = user_id.min_max(auth.user_id);

    use q::{Parameters, Params};

    let params = Params {
        from_id: auth.user_id,
        to_id: user_id,
        user_a_id,
        user_b_id,
        flags,
    };

    let db = state.db.write.get().await?;

    let res = db.query_opt_cached_typed(|| q::query(), &params.as_params()).await?;

    let Some(row) = res else {
        return Err(Error::NotFound);
    };

    let is_regular_user: bool = row.try_get(0)?;
    let is_not_blocked: bool = row.try_get(1)?;
    let flags: Option<FriendFlags> = row.try_get(2)?;

    if !is_not_blocked {
        return Err(Error::Blocked);
    }

    if !is_regular_user {
        return Err(Error::Unauthorized);
    }

    match flags {
        None => Err(Error::BadRequest),
        _ => Ok(()),
    }
}

mod q {
    use super::{FriendFlags, UserFlags};
    use crate::prelude::*;

    pub use schema::*;
    pub use thorn::*;

    thorn::params! {
        pub struct Params {
            pub from_id: UserId = Friends::UserAId,
            pub to_id: UserId = Friends::UserBId,
            pub user_a_id: UserId = Friends::UserAId,
            pub user_b_id: UserId = Friends::UserBId,
            pub flags: FriendFlags = Friends::Flags,
        }
    }

    thorn::tables! {
        pub struct SelectUsers {
            UserAId: Friends::UserAId,
            UserBId: Friends::UserBId,
            Flags: Friends::Flags,
            IsRegularUser: Type::BOOL,
            IsNotBlocked: Type::BOOL,
        }

        pub struct AddFriend {
            Flags: Friends::Flags,
        }
    }

    pub fn query() -> impl AnyQuery {
        let select_users =
            SelectUsers::as_query(
                Query::select()
                    .from(Users::left_join_table::<UserBlocks>().on(
                        UserBlocks::UserId.equals(Users::Id).and(UserBlocks::BlockId.equals(Params::from_id())),
                    ))
                    .exprs([
                        Params::user_a_id().alias_to(SelectUsers::UserAId),
                        Params::user_b_id().alias_to(SelectUsers::UserBId),
                        Params::flags().alias_to(SelectUsers::Flags),
                    ])
                    .expr(
                        Users::Flags
                            .bitand(UserFlags::ELEVATION.bits().lit())
                            .equals(0.lit())
                            .alias_to(SelectUsers::IsRegularUser),
                    )
                    .expr(UserBlocks::BlockedAt.is_null().alias_to(SelectUsers::IsNotBlocked))
                    .and_where(Users::Id.equals(Params::to_id()))
                    .and_where(Users::DeletedAt.is_null()),
            );

        let add_friend = AddFriend::as_query(
            Query::insert()
                .into::<Friends>()
                .cols(&[Friends::UserAId, Friends::UserBId, Friends::Flags])
                .query(
                    Query::select()
                        .from_table::<SelectUsers>()
                        .cols(&[SelectUsers::UserAId, SelectUsers::UserBId, SelectUsers::Flags])
                        .and_where(SelectUsers::IsNotBlocked.and(SelectUsers::IsRegularUser))
                        .as_value(),
                )
                .on_conflict(
                    [Friends::UserAId, Friends::UserBId],
                    DoUpdate
                        .set(Friends::Flags, Friends::Flags.bitor(1.lit()))
                        .set_default(Friends::UpdatedAt)
                        .and_where(
                            // and where not accepted
                            Friends::Flags.bitand(FriendFlags::ACCEPTED.bits().lit()).equals(0.lit()),
                        )
                        .and_where(
                            // and where the confirmation is from the other user, see note below
                            Params::flags()
                                .bitxor(Friends::Flags)
                                .bitand(FriendFlags::ADDED_BY.bits().lit())
                                .not_equals(0.lit()),
                        ),
                )
                .returning(Friends::Flags.alias_to(AddFriend::Flags)),
        );

        Query::select()
            .with(select_users.exclude())
            .with(add_friend.exclude())
            .from(SelectUsers::left_join_table::<AddFriend>().on(true.lit()))
            .cols(&[SelectUsers::IsRegularUser, SelectUsers::IsNotBlocked])
            .col(AddFriend::Flags)
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
