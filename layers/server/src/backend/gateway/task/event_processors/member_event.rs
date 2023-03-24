use std::sync::Arc;

use futures::{future::Either, TryFutureExt};
use schema::EventCode;

use crate::backend::util::encrypted_asset::encrypt_snowflake_opt;

use super::prelude::*;

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

    let member_future = match event {
        EventCode::MemberLeft | EventCode::MemberBan | EventCode::MemberUnban => Either::Left(async {
            mod user_query {
                pub use schema::*;
                pub use thorn::*;

                indexed_columns! {
                    pub enum UserColumns {
                        Users::Username,
                        Users::Discriminator,
                        Users::Flags,
                    }

                    pub enum ProfileColumns continue UserColumns {
                        Profiles::Nickname,
                        Profiles::AvatarId,
                        Profiles::Bits,
                    }
                }
            }

            let row = db
                .query_one_cached_typed(
                    || {
                        use user_query::*;

                        Query::select()
                            .cols(UserColumns::default())
                            .cols(ProfileColumns::default())
                            .from(
                                Users::left_join_table::<Profiles>()
                                    .on(Profiles::UserId.equals(Users::Id).and(Profiles::PartyId.is_null())),
                            )
                            .and_where(Users::Id.equals(Var::of(Users::Id)))
                    },
                    &[&user_id],
                )
                .await?;

            use user_query::{ProfileColumns, UserColumns};

            Ok::<Option<_>, Error>(Some(PartyMember {
                user: User {
                    id: user_id,
                    username: row.try_get(UserColumns::username())?,
                    discriminator: row.try_get(UserColumns::discriminator())?,
                    flags: UserFlags::from_bits_truncate_public(row.try_get(UserColumns::flags())?),
                    presence: None,
                    profile: match row.try_get(ProfileColumns::bits())? {
                        None => Nullable::Null,
                        Some(bits) => Nullable::Some(UserProfile {
                            bits,
                            extra: Default::default(),
                            nick: row.try_get(ProfileColumns::nickname())?,
                            avatar: encrypt_snowflake_opt(state, row.try_get(ProfileColumns::avatar_id())?).into(),
                            banner: Nullable::Undefined,
                            bio: Nullable::Undefined,
                            status: Nullable::Undefined,
                        }),
                    },
                    email: None,
                    preferences: None,
                },
                partial: PartialPartyMember {
                    roles: None,
                    flags: None,
                    joined_at: None,
                },
            }))
        }),
        EventCode::MemberJoined | EventCode::MemberUpdated => Either::Right(
            crate::backend::api::party::members::get_one_anonymous(
                state,
                db,
                party_id,
                user_id,
                crate::backend::api::party::members::MemberMode::Simple,
            )
            .map_ok(Some),
        ),
        _ => unreachable!(),
    };

    // If a user just joined a party, they need to be given information on it
    let party_future = async {
        if EventCode::MemberJoined != event {
            return Ok(None);
        }

        crate::backend::api::party::get::get_party_inner(state.clone(), db, user_id, party_id)
            .await
            .map(Some)
    };

    let (member, party): (Option<PartyMember>, _) = tokio::try_join!(member_future, party_future)?;

    // If no member was found, odds are it was just a side-effect
    // event from triggers after the member left
    let inner = match member {
        Some(member) => PartyMemberEvent { party_id, member },
        None => return Ok(()),
    };

    let msg = match event {
        EventCode::MemberUpdated => ServerMsg::new_member_update(inner),
        EventCode::MemberJoined => {
            let Some(party) = party else {
                return Err(Error::InternalErrorStatic("Member Joined to non-existent party"));
            };

            // Send user the party information
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
                    .broadcast_event(Event::new(ServerMsg::new_member_ban(inner.clone()), None)?, party_id)
                    .await;
            }

            ServerMsg::new_member_remove(inner)
        }
        EventCode::MemberUnban => ServerMsg::new_member_unban(inner),
        _ => unreachable!(),
    };

    state.gateway.broadcast_event(Event::new(msg, None)?, party_id).await;

    Ok(())
}
