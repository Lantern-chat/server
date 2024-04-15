use futures::{future::Either, StreamExt};
use smol_str::SmolStr;

use crate::util::encrypted_asset::encrypt_snowflake;

use super::prelude::*;

pub async fn profile_updated(
    state: &ServerState,
    db: &db::pool::Client,
    user_id: UserId,
    party_id: Option<PartyId>,
) -> Result<(), Error> {
    let do_update = async {
        #[rustfmt::skip]
        let mut stream = std::pin::pin!(db.query_stream2(schema::sql! {
            SELECT
                Users.Username          AS @Username,
                Users.Discriminator     AS @Discriminator,
                Users.Flags             AS @UserFlags,
                PartyMembers.PartyId    AS @PartyId,
                .combine_profile_bits(
                    BaseProfile.Bits,
                    PartyProfile.Bits,
                    PartyProfile.AvatarId
                ) AS @ProfileBits,

                COALESCE(PartyProfile.AvatarId, BaseProfile.AvatarId) AS @AvatarId,
                COALESCE(PartyProfile.Nickname, BaseProfile.Nickname) AS @Nickname,
                COALESCE(PartyProfile.CustomStatus, BaseProfile.CustomStatus) AS @CustomStatus

            FROM PartyMembers
                INNER JOIN Users ON Users.Id = PartyMembers.UserId

                LEFT JOIN Profiles AS BaseProfile
                    ON BaseProfile.UserId = Users.Id
                    AND BaseProfile.PartyId IS NULL

                LEFT JOIN Profiles AS PartyProfile
                    ON PartyProfile.UserId = Users.Id
                    AND PartyProfile.PartyId = PartyMembers.PartyId
            WHERE
                PartyMembers.UserId  = #{&user_id  as Users::Id}
            AND PartyMembers.PartyId = #{&party_id as Party::Id}
        }).await?);

        let mut last_avatar: Option<(FileId, SmolStr)> = None;

        while let Some(row_res) = stream.next().await {
            let row = row_res?;

            let party_id: Option<PartyId> = row.party_id()?;

            let user = User {
                id: user_id,
                username: row.username()?,
                discriminator: row.discriminator()?,
                flags: UserFlags::from_bits_truncate_public(row.user_flags()?),
                email: None,
                preferences: None,
                presence: None,
                profile: match row.profile_bits()? {
                    None => Nullable::Null,
                    Some(bits) => Nullable::Some(Arc::new(UserProfile {
                        bits,
                        extra: Default::default(),
                        nick: row.nickname()?,
                        // because all of these rows are from the same user, they likely have identical avatars
                        // so try to avoid the encryption work every iteration
                        avatar: match row.avatar_id()? {
                            Some(avatar_id) => Nullable::Some(match last_avatar {
                                Some((last_id, ref last_encrypted)) if last_id == avatar_id => {
                                    last_encrypted.clone()
                                }
                                _ => {
                                    let newly_encrypted = encrypt_snowflake(state, avatar_id);
                                    last_avatar = Some((avatar_id, newly_encrypted.clone()));
                                    newly_encrypted
                                }
                            }),
                            None => Nullable::Null,
                        },
                        banner: Nullable::Undefined,
                        status: row.custom_status()?,
                        bio: Nullable::Undefined,
                    })),
                },
            };

            let event = ServerMsg::new_profile_update(ProfileUpdateEvent { party_id, user });

            match party_id {
                Some(party_id) => {
                    state.gateway.events.send_simple(&ServerEvent::party(party_id, None, event)).await
                }
                None => log::error!("Unimplemented profile event"),
            }
        }

        Ok::<_, Error>(())
    };

    let self_update = match party_id.is_some() {
        true => Either::Left(futures::future::ok(())),
        false => Either::Right(super::user_event::self_update(state, db, user_id, None)),
    };

    tokio::try_join!(do_update, self_update)?;

    Ok(())
}
