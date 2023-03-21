use schema::Snowflake;
use sdk::models::*;

use crate::backend::util::encrypted_asset::encrypt_snowflake_opt;
use crate::backend::{api::perm::get_cached_room_permissions, gateway::Event};
use crate::{Authorization, Error, ServerState};

use sdk::models::gateway::message::ServerMsg;

pub async fn trigger_typing(state: ServerState, auth: Authorization, room_id: Snowflake) -> Result<(), Error> {
    let permissions = get_cached_room_permissions(&state, auth.user_id, room_id).await?;

    if !permissions.contains(Permissions::SEND_MESSAGES) {
        return Err(Error::NotFound);
    }

    let db = state.db.read.get().await?;

    mod q {
        pub use schema::*;
        pub use thorn::*;

        thorn::tables! {
            pub struct AggRoom {
                PartyId: Rooms::PartyId,
            }

            pub struct AggRoles {
                RoleIds: SNOWFLAKE_ARRAY,
            }
        }

        thorn::decl_alias! {
            pub BaseProfile = Profiles,
            pub PartyProfile = Profiles
        }

        thorn::params! {
            pub struct Params {
                pub user_id: Snowflake = Users::Id,
                pub room_id: Snowflake = Rooms::Id,
            }
        }

        thorn::indexed_columns! {
            pub enum RoomColumns {
                AggRoom::PartyId,
            }

            pub enum UserColumns continue RoomColumns {
                Users::Username,
                Users::Discriminator,
                Users::Flags,
            }

            pub enum MemberColumns continue UserColumns {
                PartyMembers::JoinedAt,
            }

            pub enum ProfileColumns continue MemberColumns {
                Profiles::Bits,
                Profiles::AvatarId,
                Profiles::Nickname,
            }

            pub enum RoleColumns continue ProfileColumns {
                AggRoles::RoleIds,
            }
        }
    }

    use q::{Parameters, Params};

    let row = db
        .query_opt_cached_typed(
            || {
                use q::*;

                // find the party_id from the given room_id
                let room_agg = AggRoom::as_query(
                    Query::select()
                        .expr(Rooms::PartyId.alias_to(AggRoom::PartyId))
                        .from_table::<Rooms>()
                        .and_where(Rooms::Id.equals(Params::room_id())),
                );

                Query::with()
                    .with(room_agg.exclude())
                    .select()
                    .cols(RoomColumns::default())
                    .cols(UserColumns::default())
                    .cols(MemberColumns::default())
                    // ProfileColumns, must follow order as listed above
                    .expr(schema::combine_profile_bits(
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
                    .cols(RoleColumns::default())
                    .from(
                        Users::left_join(
                            PartyMembers::inner_join_table::<AggRoom>()
                                .on(PartyMembers::PartyId.equals(AggRoom::PartyId)),
                        )
                        .on(PartyMembers::UserId.equals(Users::Id))
                        .left_join_table::<BaseProfile>()
                        .on(BaseProfile::col(Profiles::UserId)
                            .equals(Users::Id)
                            .and(BaseProfile::col(Profiles::PartyId).is_null()))
                        .left_join_table::<PartyProfile>()
                        .on(PartyProfile::col(Profiles::UserId)
                            .equals(Users::Id)
                            .and(PartyProfile::col(Profiles::PartyId).equals(AggRoom::PartyId)))
                        .left_join(Lateral(AggRoles::as_query(
                            Query::select()
                                .expr(Builtin::array_agg(RoleMembers::RoleId).alias_to(AggRoles::RoleIds))
                                .from(
                                    RoleMembers::inner_join_table::<Roles>().on(Roles::Id
                                        .equals(RoleMembers::RoleId)
                                        .and(Roles::PartyId.equals(AggRoom::PartyId))),
                                )
                                .and_where(RoleMembers::UserId.equals(Users::Id)),
                        )))
                        .on(true.lit()),
                    )
                    .and_where(Users::Id.equals(Params::user_id()))
            },
            &Params {
                user_id: auth.user_id,
                room_id,
            }
            .as_params(),
        )
        .await?;

    let Some(row) = row else { return Ok(()) };

    use q::{MemberColumns, ProfileColumns, RoleColumns, RoomColumns, UserColumns};

    let party_id: Option<Snowflake> = row.try_get(RoomColumns::party_id())?;

    let user = User {
        id: auth.user_id,
        username: row.try_get(UserColumns::username())?,
        discriminator: row.try_get(UserColumns::discriminator())?,
        flags: UserFlags::from_bits_truncate_public(row.try_get(UserColumns::flags())?),
        presence: None,
        email: None,
        preferences: None,
        profile: match row.try_get(ProfileColumns::bits())? {
            None => Nullable::Null,
            Some(bits) => Nullable::Some(UserProfile {
                bits,
                extra: Default::default(),
                nick: row.try_get(ProfileColumns::nickname())?,
                avatar: encrypt_snowflake_opt(&state, row.try_get(ProfileColumns::avatar_id())?).into(),
                banner: Nullable::Undefined,
                status: Nullable::Undefined,
                bio: Nullable::Undefined,
            }),
        },
    };

    match party_id {
        Some(party_id) => {
            let member = PartyMember {
                user,
                partial: PartialPartyMember {
                    roles: row.try_get(RoleColumns::role_ids())?,
                    joined_at: row.try_get(MemberColumns::joined_at())?,
                    flags: None,
                },
            };

            let event = ServerMsg::new_typing_start(events::TypingStart {
                room_id,
                user_id: auth.user_id,
                party_id: Some(party_id),
                member: Some(member),
            });

            state
                .gateway
                .broadcast_event(Event::new(event, Some(room_id))?, party_id)
                .await;
        }
        None => todo!("Typing in non-party rooms"),
    }

    Ok(())
}
