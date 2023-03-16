use futures::{Stream, StreamExt, TryStreamExt};

use db::pool::Client;
use schema::Snowflake;

use crate::{
    backend::{api::SearchMode, util::encrypted_asset::encrypt_snowflake_opt},
    Error, ServerState,
};

use sdk::models::*;

mod role_query {
    pub use schema::*;
    pub use thorn::*;

    indexed_columns! {
        pub enum RoleColumns {
            Roles::Id,
            Roles::PartyId,
            Roles::Name,
            Roles::Permissions1,
            Roles::Permissions2,
            Roles::Color,
            Roles::Position,
            Roles::Flags,
            Roles::AvatarId,
        }
    }
}

fn base_query() -> thorn::query::SelectQuery {
    use role_query::*;

    Query::select().from_table::<Roles>().cols(RoleColumns::default())
}

pub async fn get_roles_raw<'a, 'b>(
    db: &Client,
    state: &'b ServerState,
    party_id: SearchMode<'a>,
) -> Result<impl Stream<Item = Result<Role, Error>> + 'b, Error> {
    let stream = match party_id {
        SearchMode::Single(id) => db
            .query_stream_cached_typed(
                || {
                    use role_query::*;
                    base_query().and_where(Roles::PartyId.equals(Var::of(Party::Id)))
                },
                &[&id],
            )
            .await?
            .boxed(),
        SearchMode::Many(ids) => db
            .query_stream_cached_typed(
                || {
                    use role_query::*;
                    base_query().and_where(Roles::PartyId.equals(Builtin::any(Var::of(SNOWFLAKE_ARRAY))))
                },
                &[&ids],
            )
            .await?
            .boxed(),
    };

    use role_query::RoleColumns;

    Ok(stream.map(move |row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => Ok(Role {
            id: row.try_get(RoleColumns::id())?,
            party_id: row.try_get(RoleColumns::party_id())?,
            name: row.try_get(RoleColumns::name())?,
            permissions: Permissions::from_i64(
                row.try_get(RoleColumns::permissions1())?,
                row.try_get(RoleColumns::permissions2())?,
            ),
            color: row.try_get::<_, Option<i32>>(RoleColumns::color())?.map(|c| c as u32),
            position: row.try_get(RoleColumns::position())?,
            flags: row.try_get(RoleColumns::flags())?,
            avatar: encrypt_snowflake_opt(state, row.try_get(RoleColumns::avatar_id())?),
        }),
    }))
}
