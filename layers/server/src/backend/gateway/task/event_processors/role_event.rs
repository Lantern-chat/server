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

    mod role_query {
        pub use schema::*;
        pub use thorn::*;

        indexed_columns! {
            pub enum RoleColumns {
                Roles::PartyId,
                Roles::AvatarId,
                Roles::Name,
                Roles::Permissions,
                Roles::Color,
                Roles::Position,
                Roles::Flags,
            }
        }
    }

    let row = db
        .query_one_cached_typed(
            || {
                use role_query::*;

                Query::select()
                    .cols(RoleColumns::default())
                    .from_table::<Roles>()
                    .and_where(Roles::Id.equals(Var::of(Roles::Id)))
            },
            &[&role_id],
        )
        .await?;

    use role_query::RoleColumns;

    let party_id = row.try_get(RoleColumns::party_id())?;

    let role = Role {
        id: role_id,
        party_id,
        avatar: encrypt_snowflake_opt(state, row.try_get(RoleColumns::avatar_id())?),
        name: row.try_get(RoleColumns::name())?,
        permissions: Permission::unpack_i64(row.try_get(RoleColumns::permissions())?),
        color: {
            row.try_get::<_, Option<i32>>(RoleColumns::color())?
                .map(|c| c as u32)
        },
        position: row.try_get(RoleColumns::position())?,
        flags: row.try_get(RoleColumns::flags())?,
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
