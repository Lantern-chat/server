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
    super::user_event::self_update(state, db, user_id, party_id).await?;

    let mut stream = match party_id {
        None => db
            .query_stream_cached_typed(
                || {
                    use q::*;
                    base_query().and_where(AggMemberPresence::UserId.equals(Var::of(Users::Id)))
                },
                &[&user_id],
            )
            .await?
            .boxed(),
        Some(party_id) => db
            .query_stream_cached_typed(
                || {
                    use q::*;
                    base_query()
                        .and_where(AggMemberPresence::UserId.equals(Var::of(Users::Id)))
                        .and_where(AggMemberPresence::PartyId.equals(Var::of(Party::Id)))
                },
                &[&user_id, &party_id],
            )
            .await?
            .boxed(),
    };

    use q::Columns;

    while let Some(row_res) = stream.next().await {
        let row = row_res?;

        let party_id = row.try_get(Columns::party_id())?;

        let inner = UserPresenceEvent {
            user: User {
                id: user_id,
                username: row.try_get(Columns::username())?,
                discriminator: row.try_get(Columns::discriminator())?,
                flags: UserFlags::from_bits_truncate_public(row.try_get(Columns::user_flags())?),
                profile: match row.try_get(Columns::profile_bits())? {
                    None => Nullable::Null,
                    Some(bits) => Nullable::Some(UserProfile {
                        bits,
                        extra: Default::default(),
                        nick: row.try_get(Columns::nickname())?,
                        avatar: encrypt_snowflake_opt(state, row.try_get(Columns::avatar_id())?).into(),
                        banner: Nullable::Undefined,
                        status: row.try_get(Columns::custom_status())?,
                        bio: Nullable::Undefined,
                    }),
                },
                email: None,
                preferences: None,
            },
            presence: match row.try_get(Columns::updated_at())? {
                Some(updated_at) => UserPresence {
                    flags: UserPresenceFlags::from_bits_truncate_public(
                        row.try_get(Columns::presence_flags())?,
                    ),
                    updated_at: Some(updated_at),
                    activity: None,
                },
                None => UserPresence {
                    flags: UserPresenceFlags::empty(),
                    updated_at: None,
                    activity: None,
                },
            },
        };

        let event = Event::new(ServerMsg::new_presence_update(party_id, inner), None)?;

        state.gateway.broadcast_event(event, party_id).await;
    }

    Ok(())
}

mod q {
    pub use schema::*;
    pub use thorn::*;

    indexed_columns! {
        pub enum Columns {
            AggMemberPresence::Username,
            AggMemberPresence::Discriminator,
            AggMemberPresence::UserFlags,
            AggMemberPresence::PartyId,
            AggMemberPresence::ProfileBits,
            AggMemberPresence::Nickname,
            AggMemberPresence::AvatarId,
            AggMemberPresence::CustomStatus,
            AggMemberPresence::UpdatedAt,
            AggMemberPresence::PresenceFlags,
            AggMemberPresence::PresenceActivity,
        }
    }

    pub fn base_query() -> query::SelectQuery {
        Query::select()
            .from_table::<AggMemberPresence>()
            .cols(Columns::default())
    }
}
