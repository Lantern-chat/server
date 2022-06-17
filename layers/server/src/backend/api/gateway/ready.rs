use futures::TryStreamExt;
use hashbrown::HashMap;

use schema::Snowflake;
use thorn::pg::Json;

use crate::backend::{api::SearchMode, util::encrypted_asset::encrypt_snowflake_opt};
use crate::{Authorization, Error, ServerState};

//struct Associated<T> {
//    pub party_id: Snowflake,
//    pub value: T,
//}

pub async fn ready(
    state: ServerState,
    conn_id: Snowflake,
    auth: Authorization,
) -> Result<sdk::models::events::Ready, Error> {
    use sdk::models::*;

    log::trace!("Processing Ready Event for {}", auth.user_id);

    let db = state.db.read.get().await?;

    let perms_future = async {
        state.perm_cache.add_reference(auth.user_id).await;

        super::refresh::refresh_room_perms(&state, &db, auth.user_id).await
    };

    let user_future = async {
        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    Query::select()
                        .and_where(AggUsers::Id.equals(Var::of(Users::Id)))
                        .cols(&[
                            /* 0*/ AggUsers::Username,
                            /* 1*/ AggUsers::Discriminator,
                            /* 2*/ AggUsers::Flags,
                            /* 3*/ AggUsers::Email,
                            /* 4*/ AggUsers::CustomStatus,
                            /* 5*/ AggUsers::Biography,
                            /* 6*/ AggUsers::Preferences,
                            /* 7*/ AggUsers::AvatarId,
                        ])
                        .from_table::<AggUsers>()
                        .limit_n(1)
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
            avatar: encrypt_snowflake_opt(&state, row.try_get(7)?),
            status: row.try_get(4)?,
            bio: row.try_get(5)?,
            preferences: {
                let value: Option<Json<_>> = row.try_get(6)?;
                value.map(|v| v.0)
            },
        })
    };

    let parties_future = async {
        let rows = db
            .query_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    Query::select()
                        .cols(&[
                            /* 0*/ Party::Id,
                            /* 1*/ Party::OwnerId,
                            /* 2*/ Party::Name,
                            /* 3*/ Party::AvatarId,
                            /* 4*/ Party::Description,
                            /* 5*/ Party::DefaultRoom,
                        ])
                        .col(/*6*/ PartyMember::Position)
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

        let mut parties = HashMap::with_capacity(rows.len());
        let mut ids = Vec::with_capacity(rows.len());

        for row in rows {
            let id = row.try_get(0)?;

            ids.push(id);
            parties.insert(
                id,
                Party {
                    partial: PartialParty {
                        id,
                        name: row.try_get(2)?,
                        description: row.try_get(4)?,
                    },
                    owner: row.try_get(1)?,
                    security: SecurityFlags::empty(),
                    roles: Vec::new(),
                    emotes: Vec::new(),
                    avatar: encrypt_snowflake_opt(&state, row.try_get(3)?),
                    position: row.try_get(6)?,
                    default_room: row.try_get(5)?,
                },
            );
        }

        let (roles, emotes) = futures::future::join(
            async {
                crate::backend::api::party::roles::get_roles_raw(&db, &state, SearchMode::Many(&ids))
                    .await?
                    .try_collect::<Vec<_>>()
                    .await
            },
            async {
                crate::backend::api::party::emotes::get_custom_emotes_raw(&db, SearchMode::Many(&ids))
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

    // run all futures to competion, rather than quiting out after the first error as with `try_join!`
    // because `perm_cache` also takes some time to set, this avoids a possible race condition
    // and it doesn't really matter anyway, since the other two database tasks are pretty quick to fail
    let (user, parties) = match tokio::join!(user_future, parties_future, perms_future) {
        (Ok(user), Ok(parties), Ok(())) => (user, parties),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
            log::warn!("Error during ready event: {e}");

            //if failed, make sure the cache reference is cleaned up
            state.perm_cache.remove_reference(auth.user_id).await;

            return Err(e);
        }
    };

    Ok(events::Ready {
        user,
        dms: Vec::new(),
        parties,
        session: conn_id,
    })
}

/*
fn select_members() -> impl AnyQuery {
    use schema::*;

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
    use schema::*;

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
    use schema::*;

    Query::select()
}
*/
