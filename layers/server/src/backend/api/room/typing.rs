use schema::Snowflake;
use sdk::models::*;

use crate::backend::{api::perm::get_cached_room_permissions, gateway::Event};
use crate::{Authorization, Error, ServerState};

use sdk::models::gateway::message::ServerMsg;

pub async fn trigger_typing(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
) -> Result<(), Error> {
    let permissions = get_cached_room_permissions(&state, auth.user_id, room_id).await?;

    if !permissions.contains(RoomPermissions::SEND_MESSAGES) {
        return Err(Error::NotFound);
    }

    let db = state.db.read.get().await?;

    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                tables! {
                    struct AggRoom {
                        PartyId: Rooms::PartyId,
                    }

                    struct AggRoles {
                        RoleIds: SNOWFLAKE_ARRAY,
                    }
                }

                let user_id_var = Var::at(Users::Id, 1);
                let room_id_var = Var::at(Rooms::Id, 2);

                Query::with()
                    .with(
                        AggRoom::as_query(
                            Query::select()
                                .expr(Rooms::PartyId.alias_to(AggRoom::PartyId))
                                .from_table::<Rooms>()
                                .and_where(Rooms::Id.equals(room_id_var)),
                        )
                        .exclude(),
                    )
                    .select()
                    .col(AggRoom::PartyId)
                    .col(PartyMember::Nickname)
                    .cols(&[Users::Username, Users::Discriminator, Users::Flags])
                    .col(AggRoles::RoleIds)
                    .from(
                        Users::left_join(
                            PartyMember::inner_join_table::<AggRoom>()
                                .on(PartyMember::PartyId.equals(AggRoom::PartyId)),
                        )
                        .on(PartyMember::UserId.equals(Users::Id))
                        .left_join(Lateral(AggRoles::as_query(
                            Query::select()
                                .expr(Builtin::array_agg(RoleMembers::RoleId).alias_to(AggRoles::RoleIds))
                                .from(
                                    RoleMembers::inner_join_table::<Roles>().on(Roles::Id
                                        .equals(RoleMembers::RoleId)
                                        .and(Roles::PartyId.equals(AggRoom::PartyId))),
                                )
                                .and_where(RoleMembers::UserId.equals(Users::Id)),
                        )))
                        .on(true.lit()),
                    )
                    .and_where(Users::Id.equals(user_id_var))
            },
            &[&auth.user_id, &room_id],
        )
        .await?;

    let row = match row {
        None => return Ok(()),
        Some(row) => row,
    };

    let party_id: Option<Snowflake> = row.try_get(0)?;

    let user = User {
        id: auth.user_id,
        username: row.try_get(2)?,
        discriminator: row.try_get(3)?,
        flags: UserFlags::from_bits_truncate_public(row.try_get(4)?),
        email: None,
        preferences: None,
        profile: Nullable::Undefined,
    };

    match party_id {
        Some(party_id) => {
            let member = PartyMember {
                nick: row.try_get(1)?,
                user: Some(user),
                roles: row.try_get(5)?,
                presence: None,
                flags: None,
            };

            let event = ServerMsg::new_typing_start(Box::new(events::TypingStart {
                room: room_id,
                user: auth.user_id,
                party: Some(party_id),
                member: Some(member),
            }));

            state
                .gateway
                .broadcast_event(Event::new(event, Some(room_id))?, party_id)
                .await;
        }
        None => todo!("Typing in non-party rooms"),
    }

    Ok(())
}

/*
use thorn::*;
fn query() -> impl AnyQuery {
    use schema::*;

    let user_id_var = Var::at(Users::Id, 1);
    let room_id_var = Var::at(Rooms::Id, 2);

    Query::insert()
        .into::<EventLog>()
        .cols(&[EventLog::Code, EventLog::Id, EventLog::PartyId, EventLog::RoomId])
        .query(
            Query::select()
                .from(Rooms::left_join_table::<Party>().on(Party::Id.equals(Rooms::PartyId)))
                .expr(EventCode::TypingStarted)
                .expr(user_id_var)
                .expr(Party::Id)
                .expr(Rooms::Id)
                .and_where(Rooms::Id.equals(room_id_var))
                .as_value(),
        )
}
 */
