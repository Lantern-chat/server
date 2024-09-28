use sdk::models::gateway::{
    events::{PartyPositionUpdate, PartyUpdateEvent},
    message::ServerMsg,
};

use super::prelude::*;

pub async fn user_update(state: &ServerState, db: &db::Client, user_id: UserId) -> Result<(), Error> {
    let self_future = self_update(state, db, user_id, None);

    let user_future = async {
        #[rustfmt::skip]
        let row = db.query_one2(schema::sql! {
            SELECT
                Users.Username AS @_,
                Users.Discriminator AS @_,
                Users.Flags AS @_
            FROM Users WHERE Users.Id = #{&user_id as Users::Id}
        }).await?;

        Ok::<_, Error>(User {
            id: user_id,
            username: row.users_username()?,
            discriminator: row.users_discriminator()?,
            flags: UserFlags::from_bits_truncate_public(row.users_flags()?),
            presence: None,
            email: None,
            preferences: None,
            profile: Nullable::Undefined,
        })
    };

    // TODO: Use a union/view for these IDs

    // send events to these users because they are friend-like
    let friends_future = async {
        #[rustfmt::skip]
        let row = db.query_one2(schema::sql! {
            SELECT COALESCE(ARRAY_AGG(AggRelationships.FriendId), "{}") AS @FriendIds
            FROM AggRelationships WHERE AggRelationships.UserId = #{&user_id as AggRelationships::UserId}
            AND AggRelationships.RelA != 0
        }).await?;

        Ok::<Vec<UserId>, Error>(row.friend_ids()?)
    };

    let parties_future = async {
        #[rustfmt::skip]
        let row = db.query_one2(schema::sql! {
            // ARRAY_AGG may return NULL if empty, so coalesce to empty array
            SELECT COALESCE(ARRAY_AGG(PartyMembers.PartyId), "{}") AS @PartyIds
            FROM PartyMembers WHERE PartyMembers.UserId = #{&user_id as PartyMembers::UserId}
        }).await?;

        Ok::<Vec<PartyId>, Error>(row.party_ids()?)
    };

    let (_, user, friend_ids, party_ids) =
        tokio::try_join!(self_future, user_future, friends_future, parties_future)?;

    // shotgun the event to every relevant parties and users
    #[rustfmt::skip]
    state.gateway.events.send(&ServerEvent::new(
        friend_ids.into(),
        party_ids.into(),
        None,
        ServerMsg::new_user_update(user),
    ))
    .await?;

    // TODO: Also include open DMs

    Ok(())
}

pub async fn self_update(
    state: &ServerState,
    db: &db::Client,
    user_id: UserId,
    party_id: Option<PartyId>,
) -> Result<(), Error> {
    // When a party_id is present, it signifies that the event is just a position update in the party list
    if let Some(party_id) = party_id {
        return party_position_update(state, db, user_id, party_id).await;
    }

    // TODO: Move this into internal
    let user = crate::rpc::user::me::user_get_self::get_full_self(state, user_id).await?;

    #[rustfmt::skip]
    state.gateway.events.send(&ServerEvent::user(user_id, None, ServerMsg::new_user_update(user))).await?;

    Ok(())
}

async fn party_position_update(
    state: &ServerState,
    db: &db::Client,
    user_id: UserId,
    party_id: PartyId,
) -> Result<(), Error> {
    #[rustfmt::skip]
    let row = db.query_one2(schema::sql! {
        SELECT PartyMembers.Position AS @Position
          FROM PartyMembers
         WHERE PartyMembers.PartyId = #{&party_id as Party::Id}
           AND PartyMembers.UserId  = #{&user_id  as Users::Id}
    }).await?;

    let position: i16 = row.position()?;

    let event = ServerMsg::new_party_update(PartyUpdateEvent::Position(PartyPositionUpdate {
        position,
        id: party_id,
    }));

    state.gateway.events.send(&ServerEvent::user(user_id, None, event)).await?;

    Ok(())
}
