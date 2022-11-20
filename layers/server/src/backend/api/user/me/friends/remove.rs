use futures::{Stream, StreamExt};

use crate::{Authorization, Error, ServerState};

use sdk::models::*;

pub async fn remove_friend(state: ServerState, auth: Authorization, user_id: Snowflake) -> Result<(), Error> {
    let (user_a_id, user_b_id) =
        if auth.user_id < user_id { (auth.user_id, user_id) } else { (user_id, auth.user_id) };

    use q::{Parameters, Params};

    let params = Params { user_a_id, user_b_id };

    let db = state.db.write.get().await?;

    db.execute_cached_typed(|| q::query(), &params.as_params())
        .await?;

    Ok(())
}

mod q {
    pub use schema::*;
    pub use thorn::*;

    thorn::params! {
        pub struct Params {
            pub user_a_id: Snowflake = Friends::UserAId,
            pub user_b_id: Snowflake = Friends::UserBId,
        }
    }

    pub fn query() -> impl AnyQuery {
        Query::delete()
            .from::<Friends>()
            .and_where(Friends::UserAId.equals(Params::user_a_id()))
            .and_where(Friends::UserBId.equals(Params::user_b_id()))
    }
}
