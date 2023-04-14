use schema::EventCode;

use crate::backend::util::encrypted_asset::encrypt_snowflake_opt;

use super::prelude::*;

pub async fn role_event(
    state: &ServerState,
    event: EventCode,
    db: &db::pool::Client,
    role_id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    if event == EventCode::RoleDeleted {
        let party_id = match party_id {
            Some(party_id) => party_id,
            None => {
                return Err(Error::InternalError(format!(
                    "Role event without a party id!: {event:?} - {role_id}"
                )));
            }
        };

        state
            .gateway
            .broadcast_event(
                Event::new(
                    ServerMsg::new_role_delete(RoleDeleteEvent { id: role_id, party_id }),
                    None,
                )?,
                party_id,
            )
            .await;

        return Ok(());
    }

    #[rustfmt::skip]
    let row = db.query_one2(schema::sql! {
        SELECT
            Roles.PartyId       AS @PartyId,
            Roles.AvatarId      AS @AvatarId,
            Roles.Name          AS @Name,
            Roles.Permissions1  AS @Permissions1,
            Roles.Permissions2  AS @Permissions2,
            Roles.Color         AS @Color,
            Roles.Position      AS @Position,
            Roles.Flags         AS @Flags
        FROM Roles
        WHERE Roles.Id = #{&role_id => Roles::Id}
    }?).await?;

    let party_id = row.party_id()?;

    let role = Role {
        id: role_id,
        party_id,
        avatar: encrypt_snowflake_opt(state, row.avatar_id()?),
        name: row.name()?,
        permissions: Permissions::from_i64(row.permissions1()?, row.permissions2()?),
        color: row.color::<Option<i32>>()?.map(|c| c as u32),
        position: row.position()?,
        flags: row.flags()?,
    };

    let event = match event {
        EventCode::RoleCreated => ServerMsg::new_role_create(role),
        EventCode::RoleUpdated => ServerMsg::new_role_update(role),
        _ => unreachable!(),
    };

    state.gateway.broadcast_event(Event::new(event, None)?, party_id).await;

    Ok(())
}
