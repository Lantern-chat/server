use futures::{StreamExt, TryStreamExt};
use hashbrown::{hash_map::Entry, HashMap};

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, Error, SearchMode},
    permission_cache::PermMute,
    ServerState,
};

//struct Associated<T> {
//    pub party_id: Snowflake,
//    pub value: T,
//}

pub async fn ready(
    state: ServerState,
    conn_id: Snowflake,
    auth: Authorization,
) -> Result<models::ReadyEvent, Error> {
    use models::*;

    let db = state.read_db().await;

    let perms_future = async {
        if state.perm_cache.add_reference(auth.user_id).await {
            return Ok::<_, Error>(());
        }

        let stream = db
            .query_stream_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    Query::select()
                        .from_table::<AggRoomPerms>()
                        .cols(&[AggRoomPerms::RoomId, AggRoomPerms::Perms])
                        .and_where(AggRoomPerms::UserId.equals(Var::of(Users::Id)))
                },
                &[&auth.user_id],
            )
            .await?;

        let mut cache = Vec::new();

        futures::pin_mut!(stream);
        while let Some(row) = stream.next().await {
            let row = row?;

            let room_id: Snowflake = row.try_get(0)?;
            let perm: i64 = row.try_get(1)?;

            cache.push((
                room_id,
                PermMute {
                    perm: Permission::unpack(perm as u64),
                    muted: false,
                },
            ));
        }

        state.perm_cache.batch_set(auth.user_id, cache.into_iter()).await;

        Ok(())
    };

    let user_future = async {
        let row = db
            .query_one_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    Query::select()
                        .and_where(Users::Id.equals(Var::of(Users::Id)))
                        .cols(&[
                            Users::Username,      // 0
                            Users::Discriminator, // 1
                            Users::Flags,         // 2
                            Users::Email,         // 3
                            Users::CustomStatus,  // 4
                            Users::Biography,     // 5
                            Users::Preferences,   // 6
                        ])
                        .col(UserAvatars::FileId) // 7
                        .from(
                            Users::left_join_table::<UserAvatars>().on(UserAvatars::UserId.equals(Users::Id)),
                        )
                        .and_where(UserAvatars::IsMain.is_not_false())
                },
                &[&auth.user_id],
            )
            .await?;

        Ok::<_, Error>(User {
            id: auth.user_id,
            username: row.try_get(0)?,
            discriminator: row.try_get(1)?,
            flags: UserFlags::from_bits_truncate(row.try_get(2)?),
            email: Some(row.try_get(3)?),
            avatar_id: row.try_get(7)?,
            status: row.try_get(4)?,
            bio: row.try_get(5)?,
            preferences: {
                let value: Option<serde_json::Value> = row.try_get(6)?;

                match value {
                    None => None,
                    Some(v) => Some(serde_json::from_value(v)?),
                }
            },
        })
    };

    let parties_future = async {
        let rows = db
            .query_stream_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    Query::select()
                        .cols(&[
                            Party::Id,
                            Party::OwnerId,
                            Party::Name,
                            Party::AvatarId,
                            Party::Description,
                        ])
                        .from(
                            Party::left_join_table::<PartyMember>()
                                .on(PartyMember::PartyId.equals(Party::Id)),
                        )
                        .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
                        .and_where(Party::DeletedAt.is_null())
                },
                &[&auth.user_id],
            )
            .await?;

        let parties_stream = rows.map(|row| match row {
            Err(e) => Err(Error::from(e)),
            Ok(row) => Ok(Party {
                partial: PartialParty {
                    id: row.try_get(0)?,
                    name: row.try_get(2)?,
                    description: row.try_get(4)?,
                },
                owner: row.try_get(1)?,
                security: SecurityFlags::empty(),
                roles: Vec::new(),
                emotes: Vec::new(),
                icon_id: row.try_get(3)?,
            }),
        });

        let mut parties = HashMap::new();
        let mut ids = Vec::new();

        futures::pin_mut!(parties_stream);
        while let Some(res) = parties_stream.next().await {
            let party = res?;

            ids.push(party.id);
            parties.insert(party.id, party);
        }

        let (roles, emotes) = futures::future::join(
            async {
                crate::ctrl::party::roles::get_roles_raw(&state, SearchMode::Many(&ids))
                    .await?
                    .try_collect::<Vec<_>>()
                    .await
            },
            async {
                crate::ctrl::party::emotes::get_custom_emotes_raw(&state, SearchMode::Many(&ids))
                    .await?
                    .try_collect::<Vec<_>>()
                    .await
            },
        )
        .await;

        let (roles, emotes) = (roles?, emotes?);

        for role in roles {
            if let Some(party) = parties.get_mut(&role.party_id) {
                party.roles.push(role);
            }
        }

        for emote in emotes {
            if let Some(party) = parties.get_mut(&emote.party_id) {
                party.emotes.push(Emote::Custom(emote));
            }
        }

        Ok::<_, Error>(parties.into_iter().map(|(_, v)| v).collect())
    };

    let (user, parties, _) = futures::future::try_join3(user_future, parties_future, perms_future).await?;

    Ok(ReadyEvent {
        user,
        dms: Vec::new(),
        parties,
        session: conn_id,
    })
}

/*
fn select_members() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
        .and_where(PartyMember::PartyId.equals(Builtin::any(Var::of(SNOWFLAKE_ARRAY))))
        .cols(&[PartyMember::PartyId, PartyMember::Nickname])
        .cols(&[
            Users::Id,
            Users::Username,
            Users::Discriminator,
            Users::Flags,
        ])
        .col(RoleMembers::RoleId)
        .from(
            RoleMembers::right_join(
                Users::left_join_table::<PartyMember>().on(Users::Id.equals(PartyMember::UserId)),
            )
            .on(RoleMembers::UserId.equals(Users::Id)),
        )
}

fn select_members_old() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
        .and_where(PartyMember::PartyId.equals(Builtin::any(Var::of(SNOWFLAKE_ARRAY))))
        .cols(&[PartyMember::PartyId, PartyMember::Nickname])
        .cols(&[
            Users::Id,
            Users::Username,
            Users::Discriminator,
            Users::Flags,
        ])
        .expr(
            Query::select()
                .from_table::<RoleMembers>()
                .expr(Builtin::array_agg(RoleMembers::RoleId))
                .and_where(RoleMembers::UserId.equals(Users::Id))
                .as_value(),
        )
        .from(Users::left_join_table::<PartyMember>().on(Users::Id.equals(PartyMember::UserId)))
}

fn select_emotes() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
}
*/
