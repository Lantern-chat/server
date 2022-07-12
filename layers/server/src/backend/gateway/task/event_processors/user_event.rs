use thorn::pg::Json;

use crate::backend::util::encrypted_asset::encrypt_snowflake_opt;

use sdk::models::gateway::{
    events::{PartyPositionUpdate, PartyUpdateEvent},
    message::ServerMsg,
};

use super::prelude::*;

pub async fn user_update(
    state: &ServerState,
    db: &db::pool::Client,
    user_id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    self_update(state, db, user_id, None).await?;

    if let Some(party_id) = party_id {
        return user_per_party_update(state, db, user_id, party_id).await;
    }

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
            email: None,
            preferences: None,
            profile: Nullable::Undefined,
        })
    };

    let friends_future = async {
        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;

                    Query::select()
                        .expr(Builtin::array_agg_nonnull(AggFriends::FriendId))
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
        if let Some(party_id) = party_id {
            return Ok(vec![party_id]);
        }

        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;

                    Query::select()
                        .expr(Builtin::array_agg_nonnull(PartyMember::PartyId))
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

    let event = Event::new(ServerMsg::new_user_update(user), None)?;

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

async fn user_per_party_update(
    state: &ServerState,
    db: &db::pool::Client,
    user_id: Snowflake,
    party_id: Snowflake,
) -> Result<(), Error> {
    mod user_query {
        pub use schema::*;
        pub use thorn::*;

        indexed_columns! {
            pub enum UserColumns {
                Users::Username,
                Users::Discriminator,
                Users::Flags,
            }

            pub enum ProfileColumns continue UserColumns {
                AggProfiles::AvatarId,
                AggProfiles::Bits,
                AggProfiles::CustomStatus,
            }
        }
    }

    let row = db
        .query_one_cached_typed(
            || {
                use user_query::*;

                let user_id_var = Var::at(Users::Id, 1);
                let party_id_var = Var::at(Party::Id, 2);

                Query::select()
                    .cols(UserColumns::default())
                    .cols(ProfileColumns::default())
                    .from(
                        Users::left_join_table::<AggProfiles>().on(AggProfiles::UserId
                            .equals(Users::Id)
                            // profiles.party_id is allowed to be NULL
                            // NOTE: this isn't strictly necessary here, but will remain in case of changes
                            .and(AggProfiles::PartyId.equals(party_id_var).is_not_false())),
                    )
                    .and_where(Users::Id.equals(user_id_var))
            },
            &[&user_id, &party_id],
        )
        .await?;

    use user_query::{ProfileColumns, UserColumns};

    let user = User {
        id: user_id,
        username: row.try_get(UserColumns::username())?,
        discriminator: row.try_get(UserColumns::discriminator())?,
        flags: UserFlags::from_bits_truncate_public(row.try_get(UserColumns::flags())?),
        email: None,
        preferences: None,
        profile: match row.try_get(ProfileColumns::bits())? {
            None => Nullable::Null,
            Some(bits) => Nullable::Some(UserProfile {
                bits,
                avatar: encrypt_snowflake_opt(&state, row.try_get(ProfileColumns::avatar_id())?).into(),
                banner: Nullable::Undefined,
                status: row.try_get(ProfileColumns::custom_status())?,
                bio: Nullable::Undefined,
            }),
        },
    };

    let event = Event::new(ServerMsg::new_user_update(user), None)?;

    state.gateway.broadcast_event(event.clone(), party_id).await;

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

    let user = crate::backend::api::user::me::get::get_full(&state, user_id).await?;

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
