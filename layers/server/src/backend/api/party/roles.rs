use futures::{Stream, StreamExt, TryStreamExt};

use db::pool::Client;
use schema::Snowflake;

use crate::{
    backend::{api::SearchMode, util::encrypted_asset::encrypt_snowflake_opt},
    Error, ServerState,
};

use sdk::models::*;

fn base_query() -> thorn::query::SelectQuery {
    use schema::*;
    use thorn::*;

    Query::select().from_table::<Roles>().cols(&[
        /*0*/ Roles::Id,
        /*1*/ Roles::PartyId,
        /*2*/ Roles::Name,
        /*3*/ Roles::Permissions,
        /*4*/ Roles::Color,
        /*5*/ Roles::Position,
        /*6*/ Roles::Flags,
        /*7*/ Roles::AvatarId,
    ])
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

    Ok(stream.map(move |row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => Ok(Role {
            id: row.try_get(0)?,
            party_id: row.try_get(1)?,
            name: row.try_get(2)?,
            permissions: Permission::unpack(row.try_get::<_, i64>(3)? as u64),
            color: row.try_get::<_, Option<i32>>(4)?.map(|c| c as u32),
            position: row.try_get(5)?,
            flags: RoleFlags::from_bits_truncate(row.try_get(6)?),
            avatar: encrypt_snowflake_opt(&state, row.try_get(7)?),
        }),
    }))
}
