use futures::{Stream, StreamExt};

use db::pool::Client;

use crate::{
    backend::{api::SearchMode, util::encrypted_asset::encrypt_snowflake_opt},
    Error, ServerState,
};

use sdk::models::*;

pub async fn get_roles_raw<'a, 'b>(
    db: &Client,
    state: &'b ServerState,
    party_id: SearchMode<'a>,
) -> Result<impl Stream<Item = Result<Role, Error>> + 'b, Error> {
    let stream = db
        .query_stream2(schema::sql! {
            SELECT
                Roles.Id            AS @_,
                Roles.PartyId       AS @_,
                Roles.Name          AS @_,
                Roles.Permissions1  AS @_,
                Roles.Permissions2  AS @_,
                Roles.Color         AS @_,
                Roles.Position      AS @_,
                Roles.Flags         AS @_,
                Roles.AvatarId      AS @_
            FROM Roles WHERE match party_id {
                SearchMode::Single(ref id) => { Roles.PartyId =     #{id  as SNOWFLAKE} },
                SearchMode::Many(ref ids)  => { Roles.PartyId = ANY(#{ids as SNOWFLAKE_ARRAY}) },
            }
        })
        .await?;

    Ok(stream.map(move |row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => Ok(Role {
            id: row.roles_id()?,
            party_id: row.roles_party_id()?,
            name: row.roles_name()?,
            permissions: Permissions::from_i64(row.roles_permissions1()?, row.roles_permissions2()?),
            color: row.roles_color::<Option<i32>>()?.map(|c| c as u32),
            position: row.roles_position()?,
            flags: row.roles_flags()?,
            avatar: encrypt_snowflake_opt(state, row.roles_avatar_id()?),
        }),
    }))
}
