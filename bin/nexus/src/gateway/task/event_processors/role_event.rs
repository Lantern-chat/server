use schema::EventCode;

use crate::util::encrypted_asset::encrypt_snowflake_opt;

use super::prelude::*;

pub async fn role_event(
    state: &ServerState,
    event: EventCode,
    db: &db::Client,
    role_id: RoleId,
    party_id: Option<PartyId>,
) -> Result<(), Error> {
    if event == EventCode::RoleDeleted {
        let Some(party_id) = party_id else {
            return Err(Error::InternalError(format!(
                "Role event without a party id!: {event:?} - {role_id}"
            )));
        };

        #[rustfmt::skip]
        state.gateway.events.send(&ServerEvent::party(
            party_id,
            None,
            ServerMsg::new_role_delete(RoleDeleteEvent { id: role_id, party_id }),
        )).await?;

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
        WHERE Roles.Id = #{&role_id as Roles::Id}
    }).await?;

    let party_id = row.party_id()?;

    let role = Role {
        id: role_id,
        party_id,
        avatar: encrypt_snowflake_opt(state, row.avatar_id()?),
        name: row.name()?,
        desc: None, // TODO
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

    state.gateway.events.send(&ServerEvent::party(party_id, None, event)).await?;

    Ok(())
}
