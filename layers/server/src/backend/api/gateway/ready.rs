use futures::TryStreamExt;
use hashbrown::HashMap;

use schema::Snowflake;
use thorn::pg::Json;

use crate::backend::{
    api::SearchMode,
    util::encrypted_asset::{encrypt_snowflake, encrypt_snowflake_opt},
};
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

    let user_future = crate::backend::api::user::me::get::get_full_self(&state, auth.user_id);

    let parties_future = async {
        #[rustfmt::skip]
        let rows = db.query2(schema::sql! {
            SELECT
                Party.Id                AS @Id,
                Party.OwnerId           AS @OwnerId,
                Party.Name              AS @Name,
                Party.AvatarId          AS @AvatarId,
                Party.BannerId          AS @BannerId,
                Party.Description       AS @Description,
                Party.DefaultRoom       AS @DefaultRoom,
                PartyMembers.Position   AS @Position
            FROM
                Party INNER JOIN PartyMembers ON PartyMembers.PartyId = Party.Id
            WHERE
                Party.DeletedAt IS NULL
                AND PartyMembers.UserId = #{&auth.user_id => Users::Id}
        }?).await?;

        let mut parties = HashMap::with_capacity(rows.len());
        let mut ids = Vec::with_capacity(rows.len());

        for row in rows {
            let id = row.id()?;

            ids.push(id);
            parties.insert(
                id,
                Party {
                    partial: PartialParty {
                        id,
                        name: row.name()?,
                        description: row.description()?,
                    },
                    avatar: encrypt_snowflake_opt(&state, row.avatar_id()?),
                    banner: row.banner_id::<Nullable<_>>()?.map(|id| encrypt_snowflake(&state, id)),
                    default_room: row.default_room()?,
                    position: row.position()?,
                    security: SecurityFlags::empty(),
                    owner: row.owner_id()?,
                    roles: Vec::new(),
                    emotes: Vec::new(),
                    pin_folders: Vec::new(),
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
