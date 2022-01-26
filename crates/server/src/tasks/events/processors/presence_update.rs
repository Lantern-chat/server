use std::sync::Arc;

use crate::web::gateway::Event;

use sdk::models::gateway::{events::UserPresenceEvent, message::ServerMsg};

use super::*;

pub async fn presence_updated(
    state: &ServerState,
    db: &db::pool::Client,
    id: Snowflake,
) -> Result<(), Error> {
    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;

                tables! {
                    struct UserParties {
                        UserId: PartyMember::UserId,
                        PartyIds: SNOWFLAKE_ARRAY,
                    }
                }

                Query::with()
                    .with(
                        UserParties::as_query(
                            Query::select()
                                .from_table::<PartyMember>()
                                .expr(PartyMember::UserId.alias_to(UserParties::UserId))
                                .expr(
                                    Builtin::array_agg(PartyMember::PartyId).alias_to(UserParties::PartyIds),
                                )
                                .group_by(PartyMember::UserId),
                        )
                        .exclude(),
                    )
                    .select()
                    .from(
                        UserParties::inner_join_table::<Users>()
                            .on(Users::Id.equals(UserParties::UserId))
                            .left_join_table::<UserPresence>()
                            .on(UserParties::UserId.equals(UserPresence::UserId)),
                    )
                    .cols(&[
                        /* 0 */ Users::Username,
                        /* 1 */ Users::Discriminator,
                        /* 2 */ Users::Flags,
                    ])
                    .cols(&[
                        /* 3 */ UserPresence::UpdatedAt,
                        /* 4 */ UserPresence::Flags,
                        /* 5 */ UserPresence::Activity,
                    ])
                    .col(/* 6 */ UserParties::PartyIds)
                    .and_where(UserParties::UserId.equals(Var::of(Users::Id)))
                    .order_by(UserPresence::UpdatedAt.descending())
                    .limit_n(1)
            },
            &[&id],
        )
        .await?;

    let row = match row {
        Some(row) => row,
        None => return Ok(()),
    };

    let party_ids: Vec<Snowflake> = row.try_get(6)?;

    let presence = match row.try_get::<_, Option<_>>(3)? {
        Some(updated_at) => {
            let flags = UserPresenceFlags::from_bits_truncate(row.try_get(4)?);
            let activity = None; // TODO

            UserPresence {
                flags,
                updated_at: Some(updated_at),
                activity,
            }
        }
        None => UserPresence {
            flags: UserPresenceFlags::empty(),
            updated_at: None,
            activity: None,
        },
    };

    let user = User {
        id,
        username: row.try_get(0)?,
        discriminator: row.try_get(1)?,
        flags: UserFlags::from_bits_truncate(row.try_get(2)?).publicize(),
        status: None,
        bio: None,
        email: None,
        preferences: None,
        avatar: None,
    };

    let inner = Arc::new(UserPresenceEvent { user, presence });

    for party_id in party_ids {
        state
            .gateway
            .broadcast_event(
                Event::new(ServerMsg::new_presenceupdate(Some(party_id), inner.clone()), None)?,
                party_id,
            )
            .await;
    }

    Ok(())
}
