use schema::Snowflake;
use sdk::models::*;

use crate::backend::util::encrypted_asset::encrypt_snowflake_opt;
use crate::backend::{api::perm::get_cached_room_permissions, gateway::Event};
use crate::{Authorization, Error, ServerState};

use sdk::models::gateway::message::ServerMsg;

pub async fn trigger_typing(state: ServerState, auth: Authorization, room_id: Snowflake) -> Result<(), Error> {
    let permissions = get_cached_room_permissions(&state, auth.user_id, room_id).await?;

    if !permissions.contains(Permissions::SEND_MESSAGES) {
        return Err(Error::NotFound);
    }

    let db = state.db.read.get().await?;

    #[rustfmt::skip]
    let row = db.query_opt2(thorn::sql! {
        use schema::*;

        tables! {
            pub struct AggRoom {
                PartyId: Rooms::PartyId,
            }
        };

        WITH AggRoom AS (
            SELECT Rooms.PartyId AS AggRoom.PartyId
            FROM Rooms WHERE Rooms.Id = #{&room_id => SNOWFLAKE}
        )

        SELECT
            AggRoom.PartyId         AS @PartyId,
            Users.Username          AS @Username,
            Users.Discriminator     AS @Discriminator,
            Users.Flags             AS @UserFlags,
            PartyMembers.JoinedAt   AS @JoinedAt,

            .combine_profile_bits(
                BaseProfile.Bits,
                PartyProfile.Bits,
                PartyProfile.AvatarId
            ) AS @ProfileBits,

            COALESCE(PartyProfile.AvatarId, BaseProfile.AvatarId) AS @AvatarId,
            COALESCE(PartyProfile.Nickname, BaseProfile.Nickname) AS @Nickname,

            (
                SELECT ARRAY_AGG(RoleMembers.RoleId)
                FROM RoleMembers INNER JOIN Roles
                    ON  Roles.Id = RoleMembers.RoleId
                    AND Roles.PartyId = AggRoom.PartyId
            ) AS @RoleIds

            FROM
                Users LEFT JOIN PartyMembers
                    INNER JOIN AggRoom ON AggRoom.PartyId = PartyMembers.PartyId
                ON PartyMembers.UserId = Users.Id

                LEFT JOIN Profiles AS BaseProfile
                    ON BaseProfile.UserId = Users.Id
                    AND BaseProfile.PartyId IS NULL

                LEFT JOIN Profiles AS PartyProfile
                    ON PartyProfile.UserId = Users.Id
                    AND PartyProfile.PartyId = PartyMembers.PartyId

            WHERE Users.Id = #{&auth.user_id => SNOWFLAKE}
    }?).await?;

    let Some(row) = row else { return Ok(()) };

    let party_id: Option<Snowflake> = row.party_id()?;

    let user = User {
        id: auth.user_id,
        username: row.username()?,
        discriminator: row.discriminator()?,
        flags: UserFlags::from_bits_truncate_public(row.user_flags()?),
        presence: None,
        email: None,
        preferences: None,
        profile: match row.profile_bits()? {
            None => Nullable::Null,
            Some(bits) => Nullable::Some(UserProfile {
                bits,
                extra: Default::default(),
                nick: row.nickname()?,
                avatar: encrypt_snowflake_opt(&state, row.avatar_id()?).into(),
                banner: Nullable::Undefined,
                status: Nullable::Undefined,
                bio: Nullable::Undefined,
            }),
        },
    };

    match party_id {
        Some(party_id) => {
            let member = PartyMember {
                user,
                partial: PartialPartyMember {
                    roles: row.role_ids()?,
                    joined_at: row.joined_at()?,
                    flags: None,
                },
            };

            let event = ServerMsg::new_typing_start(events::TypingStart {
                room_id,
                user_id: auth.user_id,
                party_id: Some(party_id),
                member: Some(member),
            });

            state.gateway.broadcast_event(Event::new(event, Some(room_id))?, party_id).await;
        }
        None => todo!("Typing in non-party rooms"),
    }

    Ok(())
}
