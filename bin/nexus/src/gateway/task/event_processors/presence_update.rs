use futures::StreamExt;

use super::prelude::*;

pub async fn presence_updated(
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
                AggPresence.UpdatedAt   AS @UpdatedAt,
                AggPresence.Flags       AS @PresenceFlags
            FROM Users
                INNER JOIN PartyMembers ON PartyMembers.UserId = Users.Id
                LEFT JOIN AggPresence ON AggPresence.UserId = Users.Id
            WHERE
                Users.Id = #{&user_id as Users::Id}
            AND (
                PartyMembers.PartyId = #{&party_id as Party::Id}
                OR #{&party_id as Party::Id} IS NULL
            )
        }).await?);

        // TODO: Accumulate events then shotgun them or do one broadcast per iteration?
        while let Some(row_res) = stream.next().await {
            let row = row_res?;

            let party_id: Option<PartyId> = row.party_id()?;
            // TODO: Get other broadcast ids

            let inner = UserPresenceEvent {
                party_id,
                user: User {
                    id: user_id,
                    username: row.username()?,
                    discriminator: row.discriminator()?,
                    flags: UserFlags::from_bits_truncate_public(row.user_flags()?),
                    profile: Nullable::Undefined,
                    email: None,
                    preferences: None,
                    presence: Some(match row.updated_at()? {
                        Some(updated_at) => UserPresence {
                            flags: UserPresenceFlags::from_bits_truncate_public(row.presence_flags()?),
                            last_active: None, // TODO?
                            updated_at: Some(updated_at),
                            activity: None,
                        },
                        None => UserPresence {
                            flags: UserPresenceFlags::empty(),
                            last_active: None, // TODO?
                            updated_at: None,
                            activity: None,
                        },
                    }),
                },
            };

            #[rustfmt::skip]
            state.gateway.events.send_simple(&ServerEvent::new_iter(
                [], // TODO: Also send to friends
                party_id,
                None,
                ServerMsg::new_presence_update(inner),
            )).await;
        }

        Ok(())
    };

    tokio::try_join!(super::user_event::self_update(state, db, user_id, None), do_update)?;

    Ok(())
}
