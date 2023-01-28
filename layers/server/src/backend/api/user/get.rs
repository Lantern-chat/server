use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

use sdk::models::*;

pub async fn get_full_member(
    state: ServerState,
    auth: Authorization,
    user_id: Snowflake,
    party_id: Snowflake,
) -> Result<PartyMember, Error> {
    unimplemented!()
}

pub async fn get_full_user(state: ServerState, auth: Authorization, user_id: Snowflake) -> Result<User, Error> {
    let db = state.db.read.get().await?;

    use q::{columns::*, Parameters, Params};

    let params = Params {
        self_id: auth.user_id,
        user_id,
        party_id: None,
    };

    let Some(row) = db.query_opt_cached_typed(|| q::query(false), &params.as_params()).await? else {
        return Err(Error::NotFound);
    };

    let associated = auth.user_id == user_id || row.try_get(AssocColumns::associated())?;

    Ok(User {
        id: user_id,
        username: row.try_get(UserColumns::username())?,
        discriminator: row.try_get(UserColumns::discriminator())?,
        flags: UserFlags::from_bits_truncate_public(row.try_get(UserColumns::flags())?),
        email: None,
        preferences: None,
        presence: Some({
            // only show last_active to associated users
            let last_active = match associated && row.try_get(AllowLastActiveColumns::allowed())? {
                false => None,
                true => crate::backend::util::relative::approximate_relative_time(
                    &state,
                    user_id,
                    row.try_get(UserColumns::last_active())?,
                    None,
                ),
            };

            match row.try_get(PresenceColumns::updated_at())? {
                Some(updated_at) if associated => UserPresence {
                    flags: UserPresenceFlags::from_bits_truncate_public(row.try_get(PresenceColumns::flags())?),
                    last_active,
                    updated_at: Some(updated_at),
                    activity: None,
                },
                _ => UserPresence {
                    flags: UserPresenceFlags::empty(),
                    last_active,
                    updated_at: None,
                    activity: None,
                },
            }
        }),
        profile: match row.try_get(ProfileColumns::bits())? {
            None => Nullable::Null,
            Some(bits) => Nullable::Some(UserProfile {
                bits,
                extra: Default::default(),
                nick: row.try_get(ProfileColumns::nickname())?,
                avatar: encrypt_snowflake_opt(&state, row.try_get(ProfileColumns::avatar_id())?).into(),
                banner: match associated {
                    true => encrypt_snowflake_opt(&state, row.try_get(ProfileColumns::banner_id())?).into(),
                    false => Nullable::Undefined,
                },
                status: match associated {
                    true => row.try_get(ProfileColumns::custom_status())?,
                    false => Nullable::Undefined,
                },
                bio: match associated {
                    true => row.try_get(ProfileColumns::biography())?,
                    false => Nullable::Undefined,
                },
            }),
        },
    })
}

mod q {
    use sdk::models::UserPrefsFlags;

    use schema::*;
    pub use thorn::*;

    thorn::tables! {
        pub struct TempAllowLastActive {
            Allowed: Type::BOOL,
        }

        pub struct TempAssociated {
            Associated: Type::BOOL,
        }
    }

    thorn::decl_alias! {
        pub BaseProfile = Profiles,
        pub PartyProfile = Profiles
    }

    thorn::params! {
        pub struct Params {
            pub self_id: Snowflake = Users::Id,
            pub user_id: Snowflake = Users::Id,
            pub party_id: Option<Snowflake> = Party::Id,
        }
    }

    pub mod columns {
        use super::*;

        thorn::indexed_columns! {
            pub enum UserColumns {
                Users::Discriminator,
                Users::Username,
                Users::Flags,
                Users::LastActive,
            }

            pub enum PresenceColumns continue UserColumns {
                AggPresence::UpdatedAt,
                AggPresence::Flags,
                //AggPresence::Activity,
            }

            pub enum AllowLastActiveColumns continue PresenceColumns {
                TempAllowLastActive::Allowed,
            }

            pub enum ProfileColumns continue AllowLastActiveColumns {
                Profiles::Bits,
                Profiles::AvatarId,
                Profiles::BannerId,
                Profiles::Nickname,
                Profiles::CustomStatus,
                Profiles::Biography,
            }

            pub enum AssocColumns continue ProfileColumns {
                TempAssociated::Associated
            }
        }
    }

    use columns::*;

    pub fn query(member: bool) -> impl AnyQuery {
        let mut q = Query::select()
            .cols(UserColumns::default())
            .cols(PresenceColumns::default())
            // AllowLastActiveColumns
            .expr(
                // preferences/flags can be NULL, so testing (flags & bit) != bit accounts for that
                Users::Preferences
                    .json_extract("flags".lit())
                    .cast(Type::INT4)
                    .has_no_bits(UserPrefsFlags::HIDE_LAST_ACTIVE.bits().lit()),
            );

        // ProfileColumns, must follow order as listed above
        q = match member {
            false => q
                .cols(ProfileColumns::default())
                .and_where(Params::party_id().is_null()),
            true => q
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
                    PartyProfile::col(Profiles::BannerId),
                    BaseProfile::col(Profiles::BannerId),
                )))
                .expr(Builtin::coalesce((
                    PartyProfile::col(Profiles::Nickname),
                    BaseProfile::col(Profiles::Nickname),
                )))
                .expr(Builtin::coalesce((
                    PartyProfile::col(Profiles::CustomStatus),
                    BaseProfile::col(Profiles::CustomStatus),
                )))
                .expr(Builtin::coalesce((
                    PartyProfile::col(Profiles::Biography),
                    BaseProfile::col(Profiles::Biography),
                ))),
        };

        // AssocColumns
        q = q.expr(
            Query::select()
                .from_table::<AggUserAssociations>()
                .expr(1.lit())
                .and_where(AggUserAssociations::UserId.equals(Params::self_id()))
                .and_where(AggUserAssociations::OtherId.equals(Params::user_id()))
                .exists(),
        );

        q = match member {
            false => q.from(
                Users::left_join_table::<Profiles>()
                    .on(Profiles::UserId.equals(Users::Id).and(Profiles::PartyId.is_null()))
                    .left_join_table::<AggPresence>()
                    .on(AggPresence::UserId.equals(Users::Id)),
            ),
            true => q.from(
                Users::left_join_table::<BaseProfile>()
                    .on(BaseProfile::col(Profiles::UserId)
                        .equals(Users::Id)
                        .and(BaseProfile::col(Profiles::PartyId).is_null()))
                    .left_join_table::<PartyProfile>()
                    .on(PartyProfile::col(Profiles::UserId)
                        .equals(Users::Id)
                        .and(PartyProfile::col(Profiles::PartyId).equals(Params::party_id())))
                    .left_join_table::<AggPresence>()
                    .on(AggPresence::UserId.equals(Users::Id)),
            ),
        };

        q = q.and_where(Users::Id.equals(Params::user_id()));

        q
    }
}
