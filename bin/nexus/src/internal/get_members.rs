use crate::{prelude::*, util::encrypted_asset::encrypt_snowflake_opt};

use sdk::models::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberMode {
    Simple,
    Full,
}

pub async fn get_members(
    state: ServerState,
    party_id: PartyId,
    user_id: Option<UserId>,
    member_id: Option<UserId>,
    mode: MemberMode,
) -> Result<impl Stream<Item = Result<PartyMember, Error>>, Error> {
    let db = state.db.read.get().await?;

    get_members_inner(state, &db, party_id, user_id, member_id, mode).await
}

pub async fn get_one_anonymous(
    state: &ServerState,
    db: &db::Client,
    party_id: PartyId,
    member_id: UserId,
    mode: MemberMode,
) -> Result<PartyMember, Error> {
    let mut stream =
        std::pin::pin!(get_members_inner(state.clone(), db, party_id, None, Some(member_id), mode).await?);

    match stream.next().await {
        Some(first) => first,
        None => Err(Error::NotFound),
    }
}

pub async fn get_members_inner(
    state: ServerState,
    db: &db::Client,
    party_id: PartyId,
    user_id: Option<UserId>,
    member_id: Option<UserId>,
    mode: MemberMode,
) -> Result<impl Stream<Item = Result<PartyMember, Error>>, Error> {
    let stream = db
        .query_stream2(schema::sql! {
            use schema::flags::MemberFlags;

            SELECT
                AggMembersFull.UserId               AS @UserId,
                AggMembersFull.Discriminator        AS @Discriminator,
                AggMembersFull.Username             AS @Username,
                AggMembersFull.LastActive           AS @LastActive,
                AggMembersFull.UserFlags            AS @UserFlags,
                AggMembersFull.PresenceFlags        AS @PresenceFlags,
                AggMembersFull.PresenceUpdatedAt    AS @PresenceUpdatedAt,
                AggMembersFull.Nickname             AS @Nickname,
                //AggMembersFull.MemberFlags        AS @_,
                AggMembersFull.JoinedAt             AS @JoinedAt,
                AggMembersFull.ProfileBits          AS @ProfileBits,
                AggMembersFull.AvatarId             AS @AvatarId,
                AggMembersFull.CustomStatus         AS @CustomStatus,
                AggMembersFull.RoleIds              AS @RoleIds,
                //AggMembersFull.PresenceActivity     AS @PresenceActivity,

                // AggRelationships is not included if None
                match user_id {
                    None    => { 0 },
                    Some(_) => { COALESCE(AggRelationships.RelB, 0) }
                } AS @RelB,

                AggMembersFull.BannerId             AS @BannerId,

                // don't bother reading bio if simple
                match mode {
                    MemberMode::Simple => { NULL },
                    MemberMode::Full => { AggMembersFull.Biography }
                } AS @Biography

            FROM AggMembersFull

            // if not anonymous (from gateway), join with relationships
            if let Some(ref user_id) = user_id {
                LEFT JOIN AggRelationships
                 ON AggRelationships.UserId   = #{user_id as SNOWFLAKE}
                AND AggRelationships.FriendId = AggMembersFull.UserId
            }

            WHERE AggMembersFull.PartyId     = #{&party_id as SNOWFLAKE}
              AND AggMembersFull.MemberFlags & const {MemberFlags::BANNED.bits()} = 0

            // if not anonymous, double-check this user is actually a party member
            if let Some(ref user_id) = user_id {
                AND EXISTS(
                    SELECT 1 FROM PartyMembers
                    WHERE PartyMembers.UserId  = #{user_id as SNOWFLAKE}
                      AND PartyMembers.PartyId = #{&party_id as SNOWFLAKE}
                )
            }

            if let Some(ref member_id) = member_id {
                AND AggMembersFull.UserId = #{member_id as SNOWFLAKE}

                LIMIT 1
            }

        })
        .await?;

    Ok(stream.map(move |row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => Ok({
            let rel_b: UserRelationship = row.rel_b()?;

            let is_friendly = rel_b < UserRelationship::Blocked;

            let user_id = row.user_id()?;

            PartyMember {
                joined_at: row.joined_at()?,
                roles: row.role_ids()?,
                flags: PartyMemberFlags::empty(),
                user: User {
                    id: user_id,
                    username: row.username()?,
                    discriminator: row.discriminator()?,
                    flags: UserFlags::from_bits_truncate_public(row.user_flags()?),
                    presence: match is_friendly {
                        false => None,
                        true => Some({
                            let last_active = match mode == MemberMode::Full {
                                false => None,
                                true => crate::util::relative::approximate_relative_time(
                                    &state,
                                    user_id,
                                    row.last_active()?,
                                    None,
                                ),
                            };

                            match row.presence_updated_at()? {
                                None => UserPresence {
                                    flags: UserPresenceFlags::empty(),
                                    last_active,
                                    updated_at: None,
                                    activity: None,
                                },
                                Some(updated_at) => UserPresence {
                                    last_active,
                                    updated_at: Some(updated_at),
                                    flags: UserPresenceFlags::from_bits_truncate_public(row.presence_flags()?),
                                    activity: None, // row.presence_activity::<Option<_>>()?.map(AnyActivity::Any),
                                },
                            }
                        }),
                    },
                    email: None,
                    preferences: None,
                    profile: match row.profile_bits()? {
                        None => Nullable::Null,
                        Some(bits) => Nullable::Some(Arc::new(UserProfile {
                            bits,
                            extra: Default::default(),
                            nick: match is_friendly {
                                true => row.nickname()?,
                                false => Nullable::Undefined,
                            },
                            avatar: encrypt_snowflake_opt(&state, row.avatar_id()?).into(),
                            banner: match mode == MemberMode::Full && is_friendly {
                                true => encrypt_snowflake_opt(&state, row.banner_id()?).into(),
                                false => Nullable::Undefined,
                            },
                            status: match is_friendly {
                                true => row.custom_status()?,
                                false => Nullable::Undefined,
                            },
                            bio: match mode == MemberMode::Full && is_friendly {
                                true => row.biography()?,
                                false => Nullable::Undefined,
                            },
                        })),
                    },
                },
            }
        }),
    }))
}
