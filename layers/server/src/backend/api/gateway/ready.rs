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

pub struct FullReady {
    pub ready: sdk::models::events::Ready,
    pub blocked_by: Vec<Snowflake>,
}

pub async fn ready(state: ServerState, conn_id: Snowflake, auth: Authorization) -> Result<FullReady, Error> {
    use sdk::models::*;

    log::trace!("Processing Ready Event for {}", auth.user_id);

    let db = state.db.read.get().await?;

    let user = crate::backend::api::user::me::get::get_full_self_inner(&state, auth.user_id, &db).await?;

    let perms_future = async {
        state.perm_cache.add_reference(auth.user_id).await;

        super::refresh::refresh_room_perms(&state, &db, auth.user_id).await
    };

    let parties_future = async {
        #[rustfmt::skip]
        let rows = db.query2(schema::sql! {
            tables! { struct AggRoles { RoleIds: SNOWFLAKE_ARRAY } };

            SELECT
                Party.Id                AS @Id,
                Party.OwnerId           AS @OwnerId,
                Party.Flags             AS @Flags,
                Party.Name              AS @Name,
                Party.AvatarId          AS @PartyAvatarId,
                Party.BannerId          AS @PartyBannerId,
                Party.Description       AS @Description,
                Party.DefaultRoom       AS @DefaultRoom,
                PartyMembers.Position   AS @Position,
                PartyMembers.JoinedAt   AS @JoinedAt,

                // keep bits null so we can just clone the existing base profile
                CASE WHEN PartyProfile.Bits IS NULL THEN NULL ELSE .combine_profile_bits(
                    BaseProfile.Bits,
                    PartyProfile.Bits,
                    PartyProfile.AvatarId
                ) END AS @ProfileBits,

                COALESCE(PartyProfile.Nickname,     BaseProfile.Nickname)       AS @Nickname,
                COALESCE(PartyProfile.AvatarId,     BaseProfile.AvatarId)       AS @MemberAvatarId,
                COALESCE(PartyProfile.BannerId,     BaseProfile.BannerId)       AS @MemberBannerId,
                COALESCE(PartyProfile.CustomStatus, BaseProfile.CustomStatus)   AS @CustomStatus,
                COALESCE(PartyProfile.Biography,    BaseProfile.Biography)      AS @Biography,

                AggRoles.RoleIds AS @RoleIds
            FROM
                Party INNER JOIN PartyMembers
                    ON PartyMembers.PartyId = Party.Id
                LEFT JOIN Profiles AS BaseProfile
                    ON (BaseProfile.UserId = PartyMembers.UserId AND BaseProfile.PartyId IS NULL)
                LEFT JOIN Profiles AS PartyProfile
                    ON (PartyProfile.UserId = PartyMembers.UserId AND PartyProfile.PartyId = PartyMembers.PartyId)
                LEFT JOIN LATERAL (
                    SELECT ARRAY_AGG(RoleMembers.RoleId) AS AggRoles.RoleIds
                    FROM RoleMembers INNER JOIN Roles
                    ON  Roles.Id = RoleMembers.RoleId
                    AND Roles.PartyId = PartyMembers.PartyId
                    AND RoleMembers.UserId = PartyMembers.UserId
                ) AS AggRoles ON TRUE
            WHERE
                Party.DeletedAt IS NULL AND PartyMembers.UserId = #{&auth.user_id as Users::Id}
        }).await?;

        let mut parties = HashMap::with_capacity(rows.len());
        let mut ids = Vec::with_capacity(rows.len());

        for row in rows {
            use gateway::events::ReadyParty;

            let id = row.id()?;

            ids.push(id);
            parties.insert(
                id,
                ReadyParty {
                    party: Party {
                        partial: PartialParty {
                            id,
                            name: row.name()?,
                            description: row.description()?,
                        },
                        flags: row.flags()?,
                        avatar: encrypt_snowflake_opt(&state, row.party_avatar_id()?),
                        banner: row.party_banner_id::<Nullable<_>>()?.map(|id| encrypt_snowflake(&state, id)),
                        default_room: row.default_room()?,
                        position: row.position()?,
                        owner: row.owner_id()?,
                        roles: ThinVec::new(),
                        emotes: ThinVec::new(),
                        pin_folders: ThinVec::new(),
                    },
                    me: PartyMember {
                        user: User {
                            profile: match row.profile_bits()? {
                                Some(bits) => Nullable::Some(Arc::new(UserProfile {
                                    bits,
                                    extra: Default::default(),
                                    nick: row.nickname()?,
                                    avatar: encrypt_snowflake_opt(&state, row.member_avatar_id()?).into(),
                                    banner: encrypt_snowflake_opt(&state, row.member_banner_id()?).into(),
                                    status: row.custom_status()?,
                                    bio: row.biography()?,
                                })),
                                None => user.profile.clone(),
                            },
                            id: user.id,
                            username: user.username.clone(),
                            discriminator: user.discriminator,
                            flags: user.flags,
                            email: None,
                            preferences: None,
                            presence: None,
                        },
                        partial: PartialPartyMember {
                            joined_at: row.joined_at()?,
                            roles: row.role_ids()?,
                            flags: None,
                        },
                    },
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
    let parties = match tokio::join!(parties_future, perms_future) {
        (Ok(parties), Ok(())) => parties,
        (Err(e), _) | (_, Err(e)) => {
            log::warn!("Error during ready event: {e}");

            //if failed, make sure the cache reference is cleaned up
            state.perm_cache.remove_reference(auth.user_id).await;

            return Err(e);
        }
    };

    Ok(FullReady {
        ready: events::Ready {
            user,
            dms: ThinVec::new(),
            parties,
            session: conn_id,
        },
        blocked_by: Vec::new(),
    })
}
