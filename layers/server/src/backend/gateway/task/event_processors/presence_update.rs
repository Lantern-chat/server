use futures::StreamExt;
use std::sync::Arc;

use crate::backend::util::encrypted_asset::encrypt_snowflake_opt;

use super::prelude::*;

pub async fn presence_updated(
    state: &ServerState,
    db: &db::pool::Client,
    user_id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let do_update = async {
        use q::{Parameters, Params, PartyColumns, PresenceColumns, UserColumns};

        let params = Params { user_id, party_id };

        #[rustfmt::skip]
        let stream = db.query_stream_cached_typed(|| q::query(), &params.as_params()).await?;

        futures::pin_mut!(stream);

        // TODO: Accumulate events then shotgun them or do one broadcast per iteration?
        while let Some(row_res) = stream.next().await {
            let row = row_res?;

            let party_id: Option<Snowflake> = row.try_get(PartyColumns::party_id())?;
            // TODO: Get other broadcast ids

            let inner = UserPresenceEvent {
                party_id,
                user: User {
                    id: user_id,
                    username: row.try_get(UserColumns::username())?,
                    discriminator: row.try_get(UserColumns::discriminator())?,
                    flags: UserFlags::from_bits_truncate_public(row.try_get(UserColumns::flags())?),
                    profile: Nullable::Undefined,
                    email: None,
                    preferences: None,
                    presence: Some(match row.try_get(PresenceColumns::updated_at())? {
                        Some(updated_at) => UserPresence {
                            flags: UserPresenceFlags::from_bits_truncate_public(
                                row.try_get(PresenceColumns::flags())?,
                            ),
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

            if let Some(party_id) = party_id {
                let event = Event::new(ServerMsg::new_presence_update(inner), None)?;

                state.gateway.broadcast_event(event, party_id).await;
            }
        }

        Ok(())
    };

    tokio::try_join!(super::user_event::self_update(state, db, user_id, None), do_update)?;

    Ok(())
}

mod q {
    pub use schema::*;
    pub use thorn::*;

    thorn::params! {
        pub struct Params {
            pub user_id: Snowflake = Users::Id,
            pub party_id: Option<Snowflake> = PartyMember::PartyId,
        }
    }

    thorn::indexed_columns! {
        pub enum UserColumns {
            Users::Username,
            Users::Discriminator,
            Users::Flags,
        }

        pub enum PartyColumns continue UserColumns {
            PartyMember::PartyId,
        }

        pub enum PresenceColumns continue PartyColumns {
            AggPresence::UpdatedAt,
            AggPresence::Flags,
            //AggPresence::Activity,
        }
    }

    pub fn query() -> query::SelectQuery {
        Query::select()
            .cols(UserColumns::default())
            .cols(PartyColumns::default())
            .cols(PresenceColumns::default())
            .and_where(Users::Id.equals(Params::user_id()))
            .and_where(
                PartyMember::PartyId
                    .equals(Params::party_id())
                    .or(Params::party_id().is_null()),
            )
            .from(
                Users::inner_join_table::<PartyMember>()
                    .on(PartyMember::UserId.equals(Users::Id))
                    .left_join_table::<AggPresence>()
                    .on(AggPresence::UserId.equals(Users::Id)),
            )
    }
}
