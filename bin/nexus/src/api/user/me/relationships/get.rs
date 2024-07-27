use crate::{prelude::*, util::encrypted_asset::encrypt_snowflake_opt};

use sdk::models::*;

pub async fn get_relationships(
    state: ServerState,
    auth: Authorization,
) -> Result<impl Stream<Item = Result<Relationship, Error>>, Error> {
    let db = state.db.read.get().await?;

    #[rustfmt::skip]
    let stream = db.query_stream2(schema::sql! {
        use sdk::models::UserRelationship;

        const ${ assert!(!Columns::IS_DYNAMIC); }

        SELECT
            AggRelationships.FriendId AS @FriendId,
            AggRelationships.UpdatedAt AS @UpdatedAt,
            AggRelationships.RelA AS @RelA,
            AggRelationships.RelB AS @RelB,
            AggRelationships.Note AS @Note,

            AggUsers.Username AS @Username,
            AggUsers.Flags AS @Flags,
            AggUsers.Discriminator AS @Discriminator,
            AggUsers.LastActive AS @LastActive,
            AggUsers.PresenceFlags AS @PresenceFlags,
            AggUsers.PresenceUpdatedAt AS @PresenceUpdatedAt,
            // AggUsers::PresenceActivity AS @PresenceActivity,

            Profiles.Nickname AS @Nickname,
            Profiles.AvatarId AS @AvatarId,
            Profiles.Bits AS @ProfileBits,
            Profiles.CustomStatus AS @CustomStatus,

            EXISTS(SELECT FROM AggUserAssociations
                WHERE AggUserAssociations.UserId = AggRelationships.UserId
                  AND AggUserAssociations.OtherId = AggRelationships.FriendId
            ) AS @Associated

        FROM AggRelationships INNER JOIN AggUsers ON AggUsers.Id = AggRelationships.FriendId
        LEFT JOIN Profiles ON Profiles.UserId = AggRelationships.FriendId AND Profiles.PartyId IS NULL

        WHERE AggRelationships.UserId = #{auth.user_id_ref() as AggRelationships::UserId}
          // where the other user has not blocked this one
          AND AggRelationships.RelB < const {UserRelationship::Blocked as i8}
          // where b < 2 && !((a == b) && (a == 0)), meaning it should filter None relationships, only
          // allowing friends, pending, and users blocked by ourselves
          AND NOT (AggRelationships.RelA = AggRelationships.RelB AND AggRelationships.RelA = 0)
    }).await?;

    Ok(stream.map(move |res| match res {
        Err(e) => Err(e.into()),
        Ok(row) => Ok({
            // NOTE: The query only returns users we have not blocked, so no additional
            // logic is necessary to merge associated with blocked
            let associated: bool = row.associated()?;
            let rel = row.rel_a()?;
            let user_id = row.friend_id()?;

            Relationship {
                note: row.note()?,
                since: row.updated_at()?,
                rel,
                pending: {
                    let rel_b: UserRelationship = row.rel_b()?;

                    matches!((rel, rel_b), (UserRelationship::Friend, UserRelationship::None))
                },
                user: User {
                    id: user_id,
                    username: row.username()?,
                    flags: UserFlags::from_bits_truncate_public(row.flags()?),
                    discriminator: row.discriminator()?,
                    presence: match associated {
                        // such is the case where you are sending a friend request to a non-associated user
                        // still can't view their presence until it's accepted or a party is shared
                        false => None,
                        true => Some({
                            let last_active = crate::util::relative::approximate_relative_time(
                                &state,
                                user_id,
                                row.last_active()?,
                                None,
                            );

                            match row.presence_updated_at()? {
                                Some(updated_at) => UserPresence {
                                    flags: UserPresenceFlags::from_bits_truncate_public(row.presence_flags()?),
                                    last_active,
                                    updated_at: Some(updated_at),
                                    activity: None,
                                },
                                None => UserPresence {
                                    flags: UserPresenceFlags::empty(),
                                    last_active,
                                    updated_at: None,
                                    activity: None,
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
                            nick: match associated {
                                true => row.nickname()?,
                                false => Nullable::Undefined,
                            },
                            status: match associated {
                                true => row.custom_status()?,
                                false => Nullable::Undefined,
                            },
                            avatar: encrypt_snowflake_opt(&state, row.avatar_id()?).into(),
                            banner: Nullable::Undefined,
                            bio: Nullable::Undefined,
                        })),
                    },
                },
            }
        }),
    }))
}
