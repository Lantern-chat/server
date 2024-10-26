use crate::{prelude::*, util::encrypted_asset::encrypt_snowflake_opt};
use common::emoji::EmoteOrEmojiId;

use sdk::{
    api::commands::all::PutReaction,
    models::{events::UserReactionEvent, gateway::message::ServerMsg, *},
};

pub async fn add_reaction(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<PutReaction>,
) -> Result<(), Error> {
    let Some(emote) = state.emoji.resolve_archived(&cmd.emote_id) else {
        return Err(Error::BadRequest);
    };

    let room_id: RoomId = cmd.room_id.into();
    let msg_id: MessageId = cmd.msg_id.into();

    let perms = state.perm_cache.get(auth.user_id(), room_id).await;

    if matches!(perms, Some(perms) if !perms.contains(Permissions::ADD_REACTIONS)) {
        return Err(Error::Unauthorized);
    }

    let reaction_id = state.sf.gen();

    #[rustfmt::skip]
    let row = state.db.write.get().await?.query_opt2(schema::sql! {
        struct Checked {
            MsgId: Reactions::MsgId,
            PartyId: Rooms::PartyId,
        }

        struct SelectedReaction {
            ReactionId: Reactions::Id,
            Count: Reactions::Count,
        }

        struct InsertedReaction {
            ReactionId: Reactions::Id,
        }

        struct InsertedReactionUser {
            UserId: ReactionUsers::UserId,
            ReactionId: ReactionUsers::ReactionId,
        }

        struct ReactionEvent {
            MsgId: Checked::MsgId,
            PartyId: Rooms::PartyId,
            Nickname: Profiles::Nickname,
            Username: Users::Username,
            Discriminator: Users::Discriminator,
            UserFlags: Users::Flags,
            AvatarId: Profiles::AvatarId,
            ProfileBits: Profiles::Bits,
            RoleIds: AggMembers::RoleIds,
            JoinedAt: AggMembers::JoinedAt,
        }

        // these CTEs rely on the previous ones to succeed, so msg id and
        // reaction ids are passed through them, becoming NULL if not forwarded
        //
        // 1. verify emote/emoji is valid for this room
        // 2. insert Reaction row and get ID
        // 3. insert ReactionUser
        // 4. fetch event data
        //
        // if step 1 fails, we should return Unauthorized
        // Step 2 shouldn't fail unless 1 fails
        // Step 3 will fail if the reaction already exists
        //   in which case step 4 will also fail
        //
        // So if steps 3-4 fail, don't do anything.


        WITH Checked AS (
            SELECT
                #{&msg_id as Messages::Id} AS Checked.MsgId,

            // if we have cached permissions, things can be much simpler
            if let Some(perms) = perms {
                match emote {
                    // verify the user sending this emote is a member of the party the emote belongs to
                    EmoteOrEmojiId::Emote(ref emote_id) if perms.contains(Permissions::USE_EXTERNAL_EMOTES) => {
                        PartyMembers.PartyId AS Checked.PartyId
                         FROM PartyMembers INNER JOIN Emotes ON Emotes.PartyId = PartyMembers.PartyId
                        WHERE PartyMembers.UserId = #{auth.user_id_ref() as Users::Id}
                          AND Emotes.Id = #{emote_id as Emotes::Id}
                    }
                    EmoteOrEmojiId::Emote(ref emote_id) => {
                        Rooms.PartyId AS Checked.PartyId
                         FROM LiveRooms AS Rooms INNER JOIN Emotes ON Rooms.PartyId = Emotes.PartyId
                        WHERE Rooms.Id = #{&room_id as Rooms::Id}
                          AND Emotes.Id = #{emote_id as Emotes::Id}
                    }
                    EmoteOrEmojiId::Emoji(_) => {
                        Rooms.PartyId AS Checked.PartyId
                         FROM LiveRooms AS Rooms
                        WHERE Rooms.Id = #{&room_id as Rooms::Id}
                    }
                }
            } else {
                // hacky, but whatever
                type Rooms = AggRoomPerms;

                match emote {
                    EmoteOrEmojiId::Emoji(_) => {
                        Rooms.PartyId AS Checked.PartyId
                         FROM AggRoomPerms AS Rooms
                        WHERE Rooms.Id     = #{&room_id as Rooms::Id}
                          AND Rooms.UserId = #{auth.user_id_ref() as Users::Id}
                    }
                    EmoteOrEmojiId::Emote(ref emote_id) => {
                        PartyMembers.PartyId AS Checked.PartyId
                        FROM PartyMembers
                            // ensure user is a member of the party they're using the emote from
                            INNER JOIN Emotes ON Emotes.PartyId = PartyMembers.PartyId
                            // join with rooms to get target party id
                            INNER JOIN AggRoomPerms AS Rooms ON Rooms.UserId = PartyMembers.UserId
                        WHERE
                            PartyMembers.UserId = #{auth.user_id_ref() as Users::Id}
                        AND Rooms.Id = #{&room_id as Rooms::Id}
                        AND Emotes.Id = #{emote_id as Emotes::Id}
                        // emote is in same party as the room we're sending to,
                        // or the user has the permissions to use external emotes
                        AND (Emotes.PartyId = Rooms.PartyId OR (
                            let use_external = Permissions::USE_EXTERNAL_EMOTES.to_i64();

                                (Rooms.Permissions1 & {use_external[0]} = {use_external[0]})
                            AND (Rooms.Permissions2 & {use_external[1]} = {use_external[1]})
                        ))
                    }
                }

                let add_reactions = Permissions::ADD_REACTIONS.to_i64();
                AND (Rooms.Permissions1 & {add_reactions[0]} = {add_reactions[0]})
                AND (Rooms.Permissions2 & {add_reactions[1]} = {add_reactions[1]})
            }
        ),

        SelectedReaction AS (
            SELECT
                Reactions.Id AS SelectedReaction.ReactionId,
                Reactions.Count AS SelectedReaction.Count
            FROM Checked INNER JOIN Reactions ON Reactions.MsgId = Checked.MsgId
            AND match emote {
                EmoteOrEmojiId::Emoji(ref emoji_id) => { Reactions.EmojiId = #{emoji_id as Reactions::EmojiId} }
                EmoteOrEmojiId::Emote(ref emote_id) => { Reactions.EmoteId = #{emote_id as Reactions::EmoteId} }
            }

            AND EXISTS (SELECT FROM LiveMessages WHERE LiveMessages.Id = Reactions.MsgId)
        ),

        InsertedReaction AS (
            INSERT INTO Reactions (Id, MsgId, EmoteId, EmojiId) (
                SELECT #{&reaction_id as Reactions::Id}, Checked.MsgId,
                match emote {
                    EmoteOrEmojiId::Emoji(ref emoji_id) => { NULL, #{emoji_id as Reactions::EmojiId} }
                    EmoteOrEmojiId::Emote(ref emote_id) => { #{emote_id as Reactions::EmoteId}, NULL }
                }
                FROM Checked LEFT JOIN SelectedReaction ON TRUE
                // prefer to not insert at all if we already have an ID
                WHERE (SelectedReaction.ReactionId IS NULL OR SelectedReaction.Count = 0)
            )
            ON CONFLICT match emote {
                // NOTE: Make sure to use ./ syntax to only print column names
                EmoteOrEmojiId::Emoji(_) => { (Reactions./MsgId, Reactions./EmojiId) }
                EmoteOrEmojiId::Emote(_) => { (Reactions./MsgId, Reactions./EmoteId) }
            }
            DO UPDATE Reactions SET (Id) = (
                // if count is zero, then reset the ID to update the timestamp
                // side-effect of this is allowing RETURNING to always work
                CASE Reactions.Count WHEN 0 THEN #{&reaction_id as Reactions::Id} ELSE Reactions.Id END
            )
            RETURNING
                Reactions.Id AS InsertedReaction.ReactionId
        ),

        InsertedReactionUser AS (
            INSERT INTO ReactionUsers (ReactionId, UserId) (
                SELECT
                    // Must choose the inserted reaction first, as that likely means the ID was updated
                    COALESCE(InsertedReaction.ReactionId, SelectedReaction.ReactionId),
                    #{auth.user_id_ref() as Users::Id}
                FROM SelectedReaction FULL OUTER JOIN InsertedReaction ON TRUE
            )
            ON CONFLICT DO NOTHING
            RETURNING
                ReactionUsers.UserId AS InsertedReactionUser.UserId,
                ReactionUsers.ReactionId AS InsertedReactionUser.ReactionId
        ),

        ReactionEvent AS (
            SELECT
                Checked.MsgId       AS ReactionEvent.MsgId,
                Checked.PartyId     AS ReactionEvent.PartyId,
                AggMembers.RoleIds  AS ReactionEvent.RoleIds,
                AggMembers.JoinedAt AS ReactionEvent.JoinedAt,
                Users.Username      AS ReactionEvent.Username,
                Users.Discriminator AS ReactionEvent.Discriminator,
                Users.Flags         AS ReactionEvent.UserFlags,
                COALESCE(PartyProfile.Nickname, BaseProfile.Nickname) AS ReactionEvent.Nickname,
                COALESCE(PartyProfile.AvatarId, BaseProfile.AvatarId) AS ReactionEvent.AvatarId,
                .combine_profile_bits(BaseProfile.Bits, PartyProfile.Bits, PartyProfile.AvatarId) AS ReactionEvent.ProfileBits
            FROM Checked
                INNER JOIN Users ON Users.Id = #{auth.user_id_ref() as Users::Id}
                INNER JOIN InsertedReactionUser ON InsertedReactionUser.UserId = Users.Id
                LEFT JOIN AggMembers ON AggMembers.UserId = Users.Id AND AggMembers.PartyId = Checked.PartyId
                LEFT JOIN Profiles AS BaseProfile ON BaseProfile.UserId = Users.Id AND BaseProfile.PartyId IS NULL
                LEFT JOIN Profiles AS PartyProfile ON PartyProfile.UserId = Users.Id AND PartyProfile.PartyId = Checked.PartyId
        )

        SELECT
            ReactionEvent.MsgId         AS @MsgId,
            ReactionEvent.PartyId       AS @PartyId,
            ReactionEvent.Nickname      AS @Nickname,
            ReactionEvent.Username      AS @Username,
            ReactionEvent.Discriminator AS @Discriminator,
            ReactionEvent.UserFlags     AS @UserFlags,
            ReactionEvent.AvatarId      AS @AvatarId,
            ReactionEvent.ProfileBits   AS @ProfileBits,
            ReactionEvent.RoleIds       AS @RoleIds,
            ReactionEvent.JoinedAt      AS @JoinedAt

        // If Checked is valid, but ReactionEvent is not,
        // then all columns will be NULL
        FROM Checked LEFT JOIN ReactionEvent ON TRUE
    }).await?;

    let Some(row) = row else {
        return Err(Error::Unauthorized);
    };

    if let Some(msg_id) = row.msg_id()? {
        let emote = match state.emoji.lookup(emote) {
            Some(emote) => emote,
            None => {
                log::error!("Error lookup up likely valid emote/emoji: {:?}", emote);
                return Ok(());
            }
        };

        let party_id = row.party_id()?;

        let event = ServerMsg::new_message_reaction_add(UserReactionEvent {
            emote,
            msg_id,
            room_id,
            party_id,
            user_id: auth.user_id(),
            member: Some(Box::new(PartyMember {
                user: User {
                    id: auth.user_id(),
                    username: row.username()?,
                    discriminator: row.discriminator()?,
                    flags: UserFlags::from_bits_truncate_public(row.user_flags()?),
                    presence: None,
                    email: None,
                    preferences: None,
                    profile: match row.profile_bits()? {
                        None => Nullable::Null,
                        Some(bits) => Nullable::Some(Arc::new(UserProfile {
                            bits,
                            extra: Default::default(),
                            nick: row.nickname()?,
                            avatar: encrypt_snowflake_opt(&state, row.avatar_id()?).into(),
                            banner: Nullable::Undefined,
                            status: Nullable::Undefined,
                            bio: Nullable::Undefined,
                        })),
                    },
                },
                roles: row.role_ids()?,
                joined_at: row.joined_at()?,
                flags: PartyMemberFlags::empty(),
            })),
        });

        state.gateway.events.send(&ServerEvent::party(party_id, Some(room_id), event)).await?;
    }

    Ok(())
}
