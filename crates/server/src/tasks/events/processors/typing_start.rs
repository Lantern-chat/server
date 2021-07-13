use crate::web::gateway::{msg::ServerMsg, Event};

use super::*;

pub async fn trigger_typing(
    state: &ServerState,
    db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
    room_id: Snowflake,
) -> Result<(), Error> {
    match party_id {
        Some(party_id) => {
            // NOTE: Most users have zero or very few roles, so optimize for the common case
            let rows = db
                .query_cached_typed(
                    || {
                        use schema::*;
                        use thorn::*;

                        Query::select()
                            .cols(&[PartyMember::Nickname])
                            .cols(&[Users::Username, Users::Discriminator, Users::Flags])
                            .cols(&[RoleMembers::RoleId])
                            .from(
                                RoleMembers::right_join(
                                    Users::inner_join(
                                        PartyMember::inner_join_table::<Rooms>()
                                            .on(PartyMember::PartyId.equals(Rooms::PartyId)),
                                    )
                                    .on(PartyMember::UserId.equals(Users::Id)),
                                )
                                .on(RoleMembers::UserId.equals(Users::Id)),
                            )
                            .and_where(Users::Id.equals(Var::of(Users::Id)))
                            .and_where(Rooms::Id.equals(Var::of(Rooms::Id)))
                    },
                    &[&id, &room_id],
                )
                .await?;

            let mut maybe_member = None;

            let mut rows = rows.into_iter();
            if let Some(row) = rows.next() {
                let mut member = PartyMember {
                    user: Some(User {
                        id,
                        username: row.try_get(1)?,
                        discriminator: row.try_get(2)?,
                        flags: UserFlags::from_bits_truncate(row.try_get(3)?).publicize(),
                        email: None,
                        preferences: None,
                        status: None,
                        bio: None,
                        avatar_id: None,
                    }),
                    nick: row.try_get(0)?,
                    roles: None,
                    presence: None,
                };

                let mut roles = Vec::new();

                if let Some(role) = row.try_get(4)? {
                    roles.push(role);
                }

                for row in rows {
                    if let Some(role) = row.try_get(4)? {
                        roles.push(role);
                    }
                }

                member.roles = if roles.is_empty() { None } else { Some(roles) };

                maybe_member = Some(member);
            } else {
                log::warn!("Typing event from user not in the room? {} {}", id, room_id);
            }

            let event = ServerMsg::new_typingstart(Box::new(events::TypingStart {
                room: room_id,
                user: id,
                party: Some(party_id),
                member: maybe_member,
            }));

            state
                .gateway
                .broadcast_event(Event::new(event, Some(room_id))?, party_id, false)
                .await;
        }
        None => {
            todo!("Find list of users this event is visible to?");
        }
    }

    Ok(())
}
