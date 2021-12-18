use std::sync::Arc;

use futures::future::Either;
use models::events::PartyPositionUpdate;
use schema::EventCode;
use thorn::pg::Json;

use crate::{
    ctrl::util::encrypted_asset::encrypt_snowflake_opt,
    web::gateway::{
        msg::{
            server::{PartyUpdateInner, RoleDeleteInner},
            ServerMsg,
        },
        Event,
    },
};

use super::*;

pub async fn user_update(
    state: &ServerState,
    db: &db::pool::Client,
    user_id: Snowflake,
) -> Result<(), Error> {
    self_update(state, db, user_id, None).await?;

    let user_future = async {
        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;

                    Query::select()
                        .and_where(AggUsers::Id.equals(Var::of(Users::Id)))
                        .cols(&[
                            /* 0*/ AggUsers::Username,
                            /* 1*/ AggUsers::Discriminator,
                            /* 2*/ AggUsers::Flags,
                            /* 3*/ AggUsers::CustomStatus,
                            /* 4*/ AggUsers::Biography,
                            /* 5*/ AggUsers::AvatarId,
                        ])
                        .from_table::<AggUsers>()
                        .limit_n(1)
                },
                &[&user_id],
            )
            .await?;

        Ok::<_, Error>(User {
            id: user_id,
            username: row.try_get(0)?,
            discriminator: row.try_get(1)?,
            flags: UserFlags::from_bits_truncate(row.try_get(2)?).publicize(),
            email: None,
            avatar: encrypt_snowflake_opt(&state, row.try_get(5)?),
            status: row.try_get(3)?,
            bio: row.try_get(4)?,
            preferences: None,
        })
    };

    let friends_future = async {
        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;

                    Query::select()
                        .expr(Builtin::array_agg(AggFriends::FriendId))
                        .from_table::<AggFriends>()
                        .and_where(AggFriends::UserId.equals(Var::of(Users::Id)))
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
                        .expr(Builtin::array_agg(PartyMember::PartyId))
                        .from_table::<PartyMember>()
                        .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
                },
                &[&user_id],
            )
            .await?;

        let party_ids: Vec<Snowflake> = row.try_get(0)?;

        Ok::<_, Error>(party_ids)
    };

    let (user, friend_ids, party_ids) = tokio::try_join!(user_future, friends_future, parties_future)?;

    let event = Event::new(ServerMsg::new_userupdate(user), None)?;

    // shotgun the event to every relevant part

    for friend_id in friend_ids {
        state.gateway.broadcast_user_event(event.clone(), friend_id).await;
    }

    for party_id in party_ids {
        state.gateway.broadcast_event(event.clone(), party_id).await;
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
    // When a party_id is present, it signifies that the event is a position update in the party list
    if let Some(party_id) = party_id {
        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;

                    Query::select()
                        .col(PartyMember::Position)
                        .from_table::<PartyMember>()
                        .and_where(PartyMember::PartyId.equals(Var::of(Party::Id)))
                        .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
                },
                &[&party_id, &user_id],
            )
            .await?;

        let position: i16 = row.try_get(0)?;

        state
            .gateway
            .broadcast_user_event(
                Event::new(
                    ServerMsg::new_partyupdate(PartyUpdateInner::Position(PartyPositionUpdate {
                        position,
                        id: party_id,
                    })),
                    None,
                )?,
                user_id,
            )
            .await;

        return Ok(());
    }

    let row = db
        .query_one_cached_typed(
            || {
                use schema::*;

                Query::select()
                    .and_where(AggUsers::Id.equals(Var::of(Users::Id)))
                    .cols(&[
                        /* 0*/ AggUsers::Username,
                        /* 1*/ AggUsers::Discriminator,
                        /* 2*/ AggUsers::Flags,
                        /* 3*/ AggUsers::Email,
                        /* 4*/ AggUsers::CustomStatus,
                        /* 5*/ AggUsers::Biography,
                        /* 6*/ AggUsers::Preferences,
                        /* 7*/ AggUsers::AvatarId,
                    ])
                    .from_table::<AggUsers>()
                    .limit_n(1)
            },
            &[&user_id],
        )
        .await?;

    let user = User {
        id: user_id,
        username: row.try_get(0)?,
        discriminator: row.try_get(1)?,
        flags: UserFlags::from_bits_truncate(row.try_get(2)?),
        email: Some(row.try_get(3)?),
        avatar: encrypt_snowflake_opt(&state, row.try_get(7)?),
        status: row.try_get(4)?,
        bio: row.try_get(5)?,
        preferences: {
            let value: Option<Json<_>> = row.try_get(6)?;
            value.map(|v| v.0)
        },
    };

    state
        .gateway
        .broadcast_user_event(Event::new(ServerMsg::new_userupdate(user), None)?, user_id)
        .await;

    Ok(())
}
