use std::sync::Arc;

use schema::EventCode;

use crate::{
    ctrl::util::encrypted_asset::encrypt_snowflake_opt,
    web::gateway::{
        msg::{
            server::{PartyMemberInner, UserPresenceInner},
            ServerMsg,
        },
        Event,
    },
};

use super::*;

pub async fn member_event(
    state: &ServerState,
    event: EventCode,
    db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let party_id = match party_id {
        Some(party_id) => party_id,
        None => {
            return Err(Error::InternalError(format!(
                "Member Event without a party id!: {:?} - {}",
                event, id
            )));
        }
    };

    // Actual PartyMember row is deleted on member_left, so fetch user directly.
    let member = if event == EventCode::MemberLeft {
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
                &[&id],
            )
            .await?;

        PartyMember {
            user: Some(User {
                id,
                username: row.try_get(0)?,
                discriminator: row.try_get(1)?,
                flags: UserFlags::from_bits_truncate(row.try_get(2)?).publicize(),
                avatar: encrypt_snowflake_opt(&state, row.try_get(3)?),
                status: row.try_get(4)?,
                bio: row.try_get(5)?,
                email: None,
                preferences: None,
            }),
            nick: None,
            roles: None,
            presence: None,
        }
    } else {
        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    crate::ctrl::party::members::select_members2()
                        .and_where(AggMembers::UserId.equals(Var::of(Users::Id)))
                },
                &[&party_id, &id],
            )
            .await?;

        PartyMember {
            user: Some(User {
                id: row.try_get(0)?,
                username: row.try_get(2)?,
                discriminator: row.try_get(1)?,
                flags: UserFlags::from_bits_truncate(row.try_get(3)?).publicize(),
                status: row.try_get(5)?,
                bio: row.try_get(4)?,
                email: None,
                preferences: None,
                avatar: encrypt_snowflake_opt(&state, row.try_get(9)?),
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
        }
    };

    let inner = Box::new(PartyMemberInner { party_id, member });

    let msg = match event {
        EventCode::MemberUpdated => ServerMsg::new_memberupdate(inner),
        EventCode::MemberJoined => ServerMsg::new_memberadd(inner),
        EventCode::MemberLeft | EventCode::MemberBan => {
            let inner: Arc<PartyMemberInner> = Arc::from(inner);

            if event == EventCode::MemberBan {
                state
                    .gateway
                    .broadcast_event(
                        Event::new(ServerMsg::new_memberban(inner.clone()), None)?,
                        party_id,
                        false,
                    )
                    .await;
            }

            ServerMsg::new_memberremove(inner)
        }
        EventCode::MemberUnban => ServerMsg::new_memberunban(inner),
        _ => unreachable!(),
    };

    state
        .gateway
        .broadcast_event(Event::new(msg, None)?, party_id, false)
        .await;

    Ok(())
}
