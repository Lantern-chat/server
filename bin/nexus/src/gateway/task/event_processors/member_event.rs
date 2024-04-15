use futures::{future::Either, TryFutureExt};
use schema::EventCode;

use crate::util::encrypted_asset::encrypt_snowflake_opt;

use super::prelude::*;

pub async fn member_event(
    state: &ServerState,
    event: EventCode,
    db: &db::pool::Client,
    user_id: UserId,
    party_id: Option<PartyId>,
) -> Result<(), Error> {
    let Some(party_id) = party_id else {
        return Err(Error::InternalError(format!(
            "Member Event without a party id!: {event:?} - {user_id}"
        )));
    };

    let member_future = match event {
        EventCode::MemberLeft | EventCode::MemberBan | EventCode::MemberUnban => Either::Left(async {
            #[rustfmt::skip]
            let row = db.query_one2(schema::sql! {
                SELECT
                    Users.Username      AS @Username,
                    Users.Discriminator AS @Discriminator,
                    Users.Flags         AS @Flags,
                    Profiles.Nickname   AS @Nickname,
                    Profiles.AvatarId   AS @AvatarId,
                    Profiles.Bits       AS @ProfileBits
                FROM
                    Users LEFT JOIN Profiles ON Users.Id = Profiles.UserId AND Profiles.PartyId IS NULL
                WHERE
                    Users.Id = #{&user_id as Users::Id}
            }).await?;

            Ok::<Option<_>, Error>(Some(PartyMember {
                user: User {
                    id: user_id,
                    username: row.username()?,
                    discriminator: row.discriminator()?,
                    flags: UserFlags::from_bits_truncate_public(row.flags()?),
                    presence: None,
                    profile: match row.profile_bits()? {
                        None => Nullable::Null,
                        Some(bits) => Nullable::Some(Arc::new(UserProfile {
                            bits,
                            extra: Default::default(),
                            nick: row.nickname()?,
                            avatar: encrypt_snowflake_opt(state, row.avatar_id()?).into(),
                            banner: Nullable::Undefined,
                            bio: Nullable::Undefined,
                            status: Nullable::Undefined,
                        })),
                    },
                    email: None,
                    preferences: None,
                },
                roles: ThinVec::new(),
                joined_at: None,
                flags: PartyMemberFlags::empty(),
            }))
        }),
        EventCode::MemberJoined | EventCode::MemberUpdated => Either::Right(
            crate::api::party::members::get_one_anonymous(
                state,
                db,
                party_id,
                user_id,
                crate::api::party::members::MemberMode::Simple,
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

        crate::api::party::get::get_party_inner(state.clone(), db, user_id, party_id).await.map(Some)
    };

    let (member, party): (Option<PartyMember>, _) = tokio::try_join!(member_future, party_future)?;

    // If no member was found, odds are it was just a side-effect
    // event from triggers after the member left
    let inner = match member {
        Some(member) => PartyMemberEvent { party_id, member },
        None => return Ok(()),
    };

    #[rustfmt::skip]
    let msg = match event {
        EventCode::MemberUpdated => ServerMsg::new_member_update(inner),
        EventCode::MemberJoined => {
            let Some(party) = party else {
                return Err(Error::InternalErrorStatic("Member Joined to non-existent party"));
            };

            // Send user the party information
            state.gateway.events.send_simple(&ServerEvent::user(user_id, None, ServerMsg::new_party_create(party))).await;

            ServerMsg::new_member_add(inner)
        }
        EventCode::MemberLeft | EventCode::MemberBan => {
            let inner: Arc<PartyMemberEvent> = Arc::new(inner);

            state.gateway.events.send_simple(&ServerEvent::user(user_id, None, ServerMsg::new_party_delete(party_id))).await;

            if event == EventCode::MemberBan {
                state.gateway.events.send_simple(&ServerEvent::party(
                    party_id,
                    None,
                    ServerMsg::new_member_ban(inner.clone()),
                )).await;
            }

            ServerMsg::new_member_remove(inner)
        }
        EventCode::MemberUnban => ServerMsg::new_member_unban(inner),
        _ => unreachable!(),
    };

    state.gateway.events.send_simple(&ServerEvent::party(party_id, None, msg)).await;

    Ok(())
}
