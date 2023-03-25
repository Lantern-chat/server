use futures::{future::Either, StreamExt};
use smol_str::SmolStr;

use crate::backend::util::encrypted_asset::encrypt_snowflake;

use super::prelude::*;

pub async fn profile_updated(
    state: &ServerState,
    db: &db::pool::Client,
    user_id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let do_update = async {
        use q::{Parameters, Params, PartyColumns, ProfileColumns, UserColumns};

        let params = q::Params { user_id, party_id };

        #[rustfmt::skip]
        let mut stream = std::pin::pin!(
            db.query_stream_cached_typed(|| q::query(), &params.as_params()).await?
        );

        let mut last_avatar: Option<(Snowflake, SmolStr)> = None;

        while let Some(row_res) = stream.next().await {
            let row = row_res?;

            let party_id: Option<Snowflake> = row.try_get(PartyColumns::party_id())?;

            let user = User {
                id: user_id,
                username: row.try_get(UserColumns::username())?,
                discriminator: row.try_get(UserColumns::discriminator())?,
                flags: UserFlags::from_bits_truncate_public(row.try_get(UserColumns::flags())?),
                email: None,
                preferences: None,
                presence: None,
                profile: match row.try_get(ProfileColumns::bits())? {
                    None => Nullable::Null,
                    Some(bits) => Nullable::Some(UserProfile {
                        bits,
                        extra: Default::default(),
                        nick: row.try_get(ProfileColumns::nickname())?,
                        // because all of these rows are from the same user, they likely have identical avatars
                        // so try to avoid the encryption work every iteration
                        avatar: match row.try_get(ProfileColumns::avatar_id())? {
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
                        status: row.try_get(ProfileColumns::custom_status())?,
                        bio: Nullable::Undefined,
                    }),
                },
            };

            let event = Event::new(
                ServerMsg::new_profile_update(ProfileUpdateEvent { party_id, user }),
                None,
            )?;

            match party_id {
                Some(party_id) => state.gateway.broadcast_event(event, party_id).await,
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

mod q {
    pub use schema::*;
    pub use thorn::*;

    thorn::indexed_columns! {
        pub enum UserColumns {
            Users::Username,
            Users::Discriminator,
            Users::Flags,
        }

        pub enum PartyColumns continue UserColumns {
            PartyMembers::PartyId,
        }

        pub enum ProfileColumns continue PartyColumns {
            Profiles::Bits,
            Profiles::AvatarId,
            Profiles::Nickname,
            Profiles::CustomStatus,
        }
    }

    thorn::decl_alias! {
        pub BaseProfile = Profiles,
        pub PartyProfile = Profiles
    }

    thorn::params! {
        pub struct Params {
            pub user_id: Snowflake = Users::Id,
            pub party_id: Option<Snowflake> = Party::Id,
        }
    }

    pub fn query() -> impl AnyQuery {
        Query::select()
            .cols(UserColumns::default())
            .cols(PartyColumns::default())
            // ProfileColumns, must follow order as listed above
            .expr(schema::combine_profile_bits::call(
                BaseProfile::col(Profiles::Bits),
                PartyProfile::col(Profiles::Bits),
                PartyProfile::col(Profiles::AvatarId),
            ))
            .expr(Builtin::coalesce((
                PartyProfile::col(Profiles::AvatarId),
                BaseProfile::col(Profiles::AvatarId),
            )))
            .expr(Builtin::coalesce((
                PartyProfile::col(Profiles::Nickname),
                BaseProfile::col(Profiles::Nickname),
            )))
            .expr(Builtin::coalesce((
                PartyProfile::col(Profiles::CustomStatus),
                BaseProfile::col(Profiles::CustomStatus),
            )))
            .and_where(PartyMembers::UserId.equals(Params::user_id()))
            .and_where(PartyMembers::PartyId.equals(Params::party_id()).or(Params::party_id().is_null()))
            .from(
                PartyMembers::inner_join_table::<Users>()
                    .on(Users::Id.equals(PartyMembers::UserId))
                    .left_join_table::<BaseProfile>()
                    .on(BaseProfile::col(Profiles::UserId)
                        .equals(Params::user_id())
                        .and(BaseProfile::col(Profiles::PartyId).is_null()))
                    .left_join_table::<PartyProfile>()
                    .on(PartyProfile::col(Profiles::UserId)
                        .equals(Params::user_id())
                        .and(PartyProfile::col(Profiles::PartyId).equals(Params::party_id()))),
            )
    }
}
