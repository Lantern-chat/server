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

    let user_future = crate::backend::api::user::me::get::get_full(&state, auth.user_id);

    let parties_future = async {
        mod party_query {
            pub use schema::*;
            pub use thorn::*;

            indexed_columns! {
                pub enum PartyColumns {
                    Party::Id,
                    Party::OwnerId,
                    Party::Name,
                    Party::AvatarId,
                    Party::Description,
                    Party::DefaultRoom,
                }

                pub enum MemberColumns continue PartyColumns {
                    PartyMember::Position
                }
            }
        }

        let rows = db
            .query_cached_typed(
                || {
                    use party_query::*;

                    Query::select()
                        .cols(PartyColumns::default())
                        .cols(MemberColumns::default())
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

        use party_query::{MemberColumns, PartyColumns};

        let mut parties = HashMap::with_capacity(rows.len());
        let mut ids = Vec::with_capacity(rows.len());

        for row in rows {
            let id = row.try_get(PartyColumns::id())?;

            ids.push(id);
            parties.insert(
                id,
                Party {
                    partial: PartialParty {
                        id,
                        name: row.try_get(PartyColumns::name())?,
                        description: row.try_get(PartyColumns::description())?,
                    },
                    owner: row.try_get(PartyColumns::owner_id())?,
                    security: SecurityFlags::empty(),
                    roles: Vec::new(),
                    emotes: Vec::new(),
                    avatar: encrypt_snowflake_opt(&state, row.try_get(PartyColumns::avatar_id())?),
                    position: row.try_get(MemberColumns::position())?,
                    default_room: row.try_get(PartyColumns::default_room())?,
                },
            );
        }

        let (roles, emotes) = tokio::try_join!(
            async {
                crate::backend::api::party::roles::get_roles_raw(&db, &state, SearchMode::Many(&ids))
                    .await?
                    .try_collect::<Vec<_>>()
                    .await
            },
            async {
                // TODO: Remove this in Ready event?
                crate::backend::api::party::emotes::get_custom_emotes_raw(&db, SearchMode::Many(&ids))
                    .await?
                    .try_collect::<Vec<_>>()
                    .await
            },
        )?;

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
