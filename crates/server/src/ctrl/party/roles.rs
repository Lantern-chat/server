use futures::{Stream, StreamExt, TryStreamExt};

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, Error, SearchMode},
    ServerState,
};

use models::*;

fn base_query() -> thorn::query::SelectQuery {
    use db::schema::*;
    use thorn::*;

    Query::select().from_table::<Roles>().cols(&[
        Roles::Id,
        Roles::PartyId,
        Roles::Name,
        Roles::Permissions,
        Roles::Color,
    ])
}

pub async fn get_roles_raw<'a>(
    state: &ServerState,
    party_id: SearchMode<'a>,
) -> Result<impl Stream<Item = Result<Role, Error>> + 'static, Error> {
    let client = state.read_db().await;

    let stream = match party_id {
        SearchMode::Single(id) => client
            .query_stream_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    base_query().and_where(Roles::PartyId.equals(Var::of(Party::Id)))
                },
                &[&id],
            )
            .await?
            .boxed(),
        SearchMode::Many(ids) => client
            .query_stream_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    base_query()
                        .and_where(Roles::PartyId.equals(Builtin::any(Var::of(Type::INT8_ARRAY))))
                },
                &[&ids],
            )
            .await?
            .boxed(),
    };

    Ok(stream.map(|row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => Ok(Role {
            id: row.try_get(0)?,
            party_id: row.try_get(1)?,
            name: row.try_get(2)?,
            admin: false,
            permissions: Permission::unpack(row.try_get::<_, i64>(3)? as u64),
            color: row.try_get::<_, i32>(4)? as u32,
            mentionable: false,
        }),
    }))
}
