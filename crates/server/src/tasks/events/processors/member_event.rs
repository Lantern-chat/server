use std::sync::Arc;

use futures::future::Either;
use schema::EventCode;

use crate::{ctrl::util::encrypted_asset::encrypt_snowflake_opt, web::gateway::Event};

use sdk::models::gateway::{
    events::{PartyMemberEvent, UserPresenceEvent},
    message::ServerMsg,
};

use super::*;

pub async fn member_event(
    state: &ServerState,
    event: EventCode,
    db: &db::pool::Client,
    user_id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let party_id = match party_id {
        Some(party_id) => party_id,
        None => {
            return Err(Error::InternalError(format!(
                "Member Event without a party id!: {event:?} - {user_id}"
            )));
        }
    };

    // Actual PartyMember row is deleted on member_left, so fetch user directly.
    let member_future = if event == EventCode::MemberLeft {
        Either::Left(async {
            let row = db
                .query_one_cached_typed(
                    || {
                        use schema::*;
                        use thorn::*;

                        Query::select()
                            .from_table::<AggUsers>()
                            .cols(&[
                                /* 0 */ AggUsers::Username,
                                /* 1 */ AggUsers::Discriminator,
                                /* 2 */ AggUsers::Flags,
                                /* 3 */ AggUsers::AvatarId,
                                /* 4 */ AggUsers::CustomStatus,
                                /* 5 */ AggUsers::Biography,
                            ])
                            .and_where(AggUsers::Id.equals(Var::of(AggUsers::Id)))
                    },
                    &[&user_id],
                )
                .await?;

            Ok::<Option<_>, Error>(Some(PartyMember {
                user: Some(User {
                    id: user_id,
                    username: row.try_get(0)?,
                    discriminator: row.try_get(1)?,
                    flags: UserFlags::from_bits_truncate(row.try_get(2)?).publicize(),
                    avatar: encrypt_snowflake_opt(state, row.try_get(3)?),
                    status: row.try_get(4)?,
                    bio: row.try_get(5)?,
                    email: None,
                    preferences: None,
                }),
                nick: None,
                roles: None,
                presence: None,
            }))
        })
    } else {
        Either::Right(async {
            let row = db
                .query_opt_cached_typed(
                    || {
                        use schema::*;
                        use thorn::*;

                        crate::ctrl::party::members::select_members2()
                            .and_where(AggMembers::UserId.equals(Var::of(Users::Id)))
                    },
                    &[&party_id, &user_id],
                )
                .await?;

            let row = match row {
                Some(row) => row,
                None => return Ok(None),
            };

            Ok::<Option<_>, Error>(Some(PartyMember {
                user: Some(User {
                    id: row.try_get(0)?,
                    username: row.try_get(2)?,
                    discriminator: row.try_get(1)?,
                    flags: UserFlags::from_bits_truncate(row.try_get(3)?).publicize(),
                    status: row.try_get(5)?,
                    bio: row.try_get(4)?,
                    email: None,
                    preferences: None,
                    avatar: encrypt_snowflake_opt(state, row.try_get(9)?),
                }),
                nick: row.try_get(10)?,
                presence: match row.try_get::<_, Option<_>>(7)? {
                    None => None,
                    Some(updated_at) => Some(UserPresence {
                        updated_at: Some(updated_at),
                        flags: UserPresenceFlags::from_bits_truncate(row.try_get(6)?),
                        activity: match row.try_get::<_, Option<serde_json::Value>>(8)? {
                            None => None,
                            Some(value) => Some(AnyActivity::Any(value)),
                        },
                    }),
                },
                roles: row.try_get(12)?,
            }))
        })
    };

    let mut party_future = Either::Left(futures::future::ok::<Option<Party>, Error>(None));

    if event == EventCode::MemberJoined {
        party_future = Either::Right(async {
            crate::ctrl::party::get::get_party_inner(state.clone(), db, user_id, party_id)
                .await
                .map(|party| Some(party))
        });
    }

    let (member, party): (Option<PartyMember>, _) = tokio::try_join!(member_future, party_future)?;

    // If no member was found, odds are it was just a side-effect
    // event from triggers after the member left
    let member = match member {
        Some(member) => member,
        None => return Ok(()),
    };

    let inner = PartyMemberEvent { party_id, member };

    let msg = match event {
        EventCode::MemberUpdated => ServerMsg::new_member_update(inner),
        EventCode::MemberJoined => {
            let party = match party {
                Some(party) => party,
                None => return Err(Error::InternalErrorStatic("Member Joined to non-existent party")),
            };

            state
                .gateway
                .broadcast_user_event(Event::new(ServerMsg::new_party_create(party), None)?, user_id)
                .await;

            ServerMsg::new_member_add(inner)
        }
        EventCode::MemberLeft | EventCode::MemberBan => {
            let inner: Arc<PartyMemberEvent> = Arc::new(inner);

            state
                .gateway
                .broadcast_user_event(Event::new(ServerMsg::new_party_delete(party_id), None)?, user_id)
                .await;

            if event == EventCode::MemberBan {
                state
                    .gateway
                    .broadcast_event(
                        Event::new(ServerMsg::new_member_ban(inner.clone()), None)?,
                        party_id,
                    )
                    .await;
            }

            ServerMsg::new_member_remove(inner)
        }
        EventCode::MemberUnban => ServerMsg::new_member_unban(inner),
        _ => unreachable!(),
    };

    state
        .gateway
        .broadcast_event(Event::new(msg, None)?, party_id)
        .await;

    Ok(())
}
