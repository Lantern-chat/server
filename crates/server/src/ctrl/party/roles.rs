use futures::{Stream, StreamExt, TryStreamExt};

use db::pool::Client;
use schema::Snowflake;

use crate::{
    ctrl::{auth::Authorization, Error, SearchMode},
    ServerState,
};

use models::*;

fn base_query() -> thorn::query::SelectQuery {
    use schema::*;
    use thorn::*;

    Query::select().from_table::<Roles>().cols(&[
        /*0*/ Roles::Id,
        /*1*/ Roles::PartyId,
        /*2*/ Roles::Name,
        /*3*/ Roles::Permissions,
        /*4*/ Roles::Color,
        /*5*/ Roles::Flags,
        /*6*/ Roles::IconId,
    ])
}

pub async fn get_roles_raw<'a>(
    db: &Client,
    party_id: SearchMode<'a>,
) -> Result<impl Stream<Item = Result<Role, Error>> + 'static, Error> {
    let stream = match party_id {
        SearchMode::Single(id) => db
            .query_stream_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    base_query().and_where(Roles::PartyId.equals(Var::of(Party::Id)))
                },
                &[&id],
            )
            .await?
            .boxed(),
        SearchMode::Many(ids) => db
            .query_stream_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    base_query().and_where(Roles::PartyId.equals(Builtin::any(Var::of(SNOWFLAKE_ARRAY))))
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
            permissions: Permission::unpack(row.try_get::<_, i64>(3)? as u64),
            color: row.try_get::<_, Option<i32>>(4)?.map(|c| c as u32),
            flags: RoleFlags::from_bits_truncate(row.try_get(5)?),
            icon_id: row.try_get(6)?,
        }),
    }))
}
