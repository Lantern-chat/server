use schema::EventCode;

use crate::backend::util::encrypted_asset::encrypt_snowflake_opt;

use sdk::models::gateway::{events::RoleDeleteEvent, message::ServerMsg};

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
                    ServerMsg::new_role_delete(RoleDeleteEvent {
                        id: role_id,
                        party_id,
                    }),
                    None,
                )?,
                party_id,
            )
            .await;

        return Ok(());
    }

    let row = db
        .query_one_cached_typed(
            || {
                use schema::*;

                Query::select()
                    .cols(&[
                        /*0*/ Roles::PartyId,
                        /*1*/ Roles::AvatarId,
                        /*2*/ Roles::Name,
                        /*3*/ Roles::Permissions,
                        /*4*/ Roles::Color,
                        /*5*/ Roles::Position,
                        /*6*/ Roles::Flags,
                    ])
                    .from_table::<Roles>()
                    .and_where(Roles::Id.equals(Var::of(Roles::Id)))
            },
            &[&role_id],
        )
        .await?;

    let party_id = row.try_get(0)?;

    let role = Role {
        id: role_id,
        party_id,
        avatar: encrypt_snowflake_opt(state, row.try_get(1)?),
        name: row.try_get(2)?,
        permissions: Permission::unpack(row.try_get::<_, i64>(3)? as u64),
        color: row.try_get::<_, Option<i32>>(4)?.map(|c| c as u32),
        position: row.try_get(5)?,
        flags: RoleFlags::from_bits_truncate(row.try_get(6)?),
    };

    let event = match event {
        EventCode::RoleCreated => ServerMsg::new_role_create(role),
        EventCode::RoleUpdated => ServerMsg::new_role_update(role),
        _ => unreachable!(),
    };

    state
        .gateway
        .broadcast_event(Event::new(event, None)?, party_id)
        .await;

    Ok(())
}
