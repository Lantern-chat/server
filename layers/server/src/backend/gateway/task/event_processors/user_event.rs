use sdk::models::gateway::{
    events::{PartyPositionUpdate, PartyUpdateEvent},
    message::ServerMsg,
};

use super::prelude::*;

pub async fn user_update(state: &ServerState, db: &db::pool::Client, user_id: Snowflake) -> Result<(), Error> {
    let self_future = self_update(state, db, user_id, None);

    let user_future = async {
        mod user_query {
            pub use schema::*;
            pub use thorn::*;

            indexed_columns! {
                pub enum UserColumns {
                    Users::Username,
                    Users::Discriminator,
                    Users::Flags,
                }
            }
        }

        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;

                    Query::select()
                        .and_where(Users::Id.equals(Var::of(Users::Id)))
                        .cols(UserColumns::default())
                        .from_table::<Users>()
                },
                &[&user_id],
            )
            .await?;

        use user_query::UserColumns;

        Ok::<_, Error>(User {
            id: user_id,
            username: row.try_get(UserColumns::username())?,
            discriminator: row.try_get(UserColumns::discriminator())?,
            flags: UserFlags::from_bits_truncate_public(row.try_get(UserColumns::flags())?),
            presence: None,
            email: None,
            preferences: None,
            profile: Nullable::Undefined,
        })
    };

    // TODO: Use a union/view for these IDs

    // send events to these users because they are friend-like
    let friends_future = async {
        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;

                    Query::select()
                        .expr(Builtin::array_agg_nonnull(AggRelationships::FriendId))
                        .from_table::<AggRelationships>()
                        .and_where(AggRelationships::UserId.equals(Var::of(Users::Id)))
                        .and_where(AggRelationships::RelA.not_equals(0.lit()))
                },
                &[&user_id],
            )
            .await?;

        let friend_ids: Vec<Snowflake> = row.try_get(0)?;

        Ok::<_, Error>(friend_ids)
    };

    let parties_future = async {
        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;

                    Query::select()
                        .expr(Builtin::array_agg_nonnull(PartyMembers::PartyId))
                        .from_table::<PartyMembers>()
                        .and_where(PartyMembers::UserId.equals(Var::of(Users::Id)))
                },
                &[&user_id],
            )
            .await?;

        let party_ids: Vec<Snowflake> = row.try_get(0)?;

        Ok::<_, Error>(party_ids)
    };

    let (_, user, friend_ids, party_ids) =
        tokio::try_join!(self_future, user_future, friends_future, parties_future)?;

    let event = Event::new(ServerMsg::new_user_update(user), None)?;

    // shotgun the event to every relevant party

    for friend_id in friend_ids {
        state.gateway.broadcast_user_event(event.clone(), friend_id).await;
    }

    for party_id in party_ids {
        state.gateway.broadcast_event(event.clone(), party_id);
    }

    // TODO: Also include open DMs

    Ok(())
}

pub async fn self_update(
    state: &ServerState,
    db: &db::pool::Client,
    user_id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    // When a party_id is present, it signifies that the event is just a position update in the party list
    if let Some(party_id) = party_id {
        return party_position_update(state, db, user_id, party_id).await;
    }

    let user = crate::backend::api::user::me::get::get_full_self(state, user_id).await?;

    state
        .gateway
        .broadcast_user_event(Event::new(ServerMsg::new_user_update(user), None)?, user_id)
        .await;

    Ok(())
}

async fn party_position_update(
    state: &ServerState,
    db: &db::pool::Client,
    user_id: Snowflake,
    party_id: Snowflake,
) -> Result<(), Error> {
    #[rustfmt::skip]
    let row = db.query_one2(schema::sql! {
        SELECT PartyMembers.Position AS @Position
          FROM PartyMembers
         WHERE PartyMembers.PartyId = #{&party_id as Party::Id}
           AND PartyMembers.UserId  = #{&user_id  as Users::Id}
    }).await?;

    let position: i16 = row.position()?;

    state
        .gateway
        .broadcast_user_event(
            Event::new(
                ServerMsg::new_party_update(PartyUpdateEvent::Position(PartyPositionUpdate {
                    position,
                    id: party_id,
                })),
                None,
            )?,
            user_id,
        )
        .await;

    Ok(())
}
