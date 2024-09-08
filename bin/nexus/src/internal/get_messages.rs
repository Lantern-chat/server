use crate::prelude::*;

use crate::util::encrypted_asset::encrypt_snowflake_opt;

use sdk::models::*;

use schema::flags::AttachmentFlags;
use thorn::pg::Json;

pub async fn get_one<DB>(state: ServerState, db: &DB, msg_id: MessageId) -> Result<Message, Error>
where
    DB: db::AnyClient,
{
    let mut stream = std::pin::pin!(get_messages(state, db, GetMsgRequest::Single { msg_id }).await?);

    match stream.next().await {
        Some(Ok(msg)) => Ok(msg),
        Some(Err(e)) => Err(e),
        None => Err(Error::NotFound),
    }
}

pub enum GetMsgRequest<'a> {
    /// Single unauthorized message
    Single { msg_id: MessageId },

    /// Many messages, possibly filtered
    Many {
        needs_perms: bool,
        user_id: UserId,
        room_id: RoomId,
        /// Note: If the cursor is `Exact`, `GetMsgRequest::Many` will still filter the messages with [`get_messages`]
        ///
        /// Consider using `GetMsgRequest::Single` instead.
        cursor: Cursor,
        parent: Option<MessageId>,
        limit: i16,
        pins: &'a [ArchivedFolderId],
        starred: bool,
        recurse: i16,
    },
}

pub async fn get_messages<'a>(
    state: ServerState,
    db: &impl db::AnyClient,
    req: GetMsgRequest<'a>,
) -> Result<impl Stream<Item = Result<Message, Error>> + 'a, Error> {
    #[rustfmt::skip]
    let stream = db.query_stream2(schema::sql! {
        tables! {
            struct SelectedMessages {
                Depth: Type::INT2,
                Id: Messages::Id,
                PartyId: Party::Id,
            }

            pub struct SortedEmbeds {
                EmbedId: MessageEmbeds::EmbedId,
                Flags: MessageEmbeds::Flags,
            }

            pub struct TempAttachments {
                Meta: Type::JSONB,
                Preview: Type::BYTEA_ARRAY,
            }

            pub struct Precalculated {
                Query: Type::TSQUERY,
            }
        };

        WITH

        match req {
            GetMsgRequest::Single { ref msg_id } => {
                SelectedMessages AS (
                    SELECT
                        Messages.Id AS SelectedMessages.Id,
                        LiveRooms.PartyId AS SelectedMessages.PartyId
                    FROM Messages INNER JOIN LiveRooms ON LiveRooms.Id = Messages.RoomId
                    WHERE Messages.Id = #{msg_id as Messages::Id}
                    LIMIT 1
                )
            }
            GetMsgRequest::Many {
                needs_perms,
                ref user_id,
                ref room_id,
                ref cursor,
                ref parent,
                ref limit,
                ref pins,
                ref recurse,
                starred
            } => {
                RECURSIVE SelectedMessages AS NOT MATERIALIZED (
                    (SELECT
                        1               AS SelectedMessages.Depth,
                        Messages.Id     AS SelectedMessages.Id,
                        Rooms.PartyId   AS SelectedMessages.PartyId
                    FROM LiveMessages   AS Messages

                    if needs_perms {
                        INNER JOIN AggRoomPerms AS Rooms ON Rooms.Id = Messages.RoomId
                            AND Rooms.UserId = #{user_id as Rooms::UserId}
                    } else {
                        INNER JOIN LiveRooms AS Rooms ON Rooms.Id = Messages.RoomId
                    }

                    if pins.len() > 1 {
                        INNER JOIN MessagePins ON MessagePins.MsgId = Messages.Id
                    }

                    WHERE Messages.RoomId = #{room_id as Rooms::Id}

                    if needs_perms {
                        type Rooms = AggRoomPerms;

                        // we know this perm is in the lower half, so only use that
                        let perms = Permissions::READ_MESSAGE_HISTORY.to_i64();
                        assert_eq!(perms[1], 0);

                        AND Rooms.Permissions1 & {perms[0]} = {perms[0]}
                    }

                    if let Some(ref parent) = parent {
                        AND Messages.ParentId = #{parent as Messages::ParentId}
                    }

                    if starred {
                        AND EXISTS (
                            SELECT FROM MessageStars
                            WHERE MessageStars.MsgId = Messages.Id
                            AND MessageStars.UserId = #{user_id as Users::Id}
                        )
                    }

                    AND match cursor {
                        Cursor::After(ref msg_id)  => { Messages.Id > #{msg_id as Messages::Id} },
                        Cursor::Before(ref msg_id) => { Messages.Id < #{msg_id as Messages::Id} },
                        Cursor::Exact(ref msg_id)  => { Messages.Id = #{msg_id as Messages::Id} }
                    }

                    use std::cmp::Ordering;

                    match pins.len().cmp(&1) {
                        Ordering::Less => {},
                        Ordering::Equal => {
                            let pin_tag = pins.first().unwrap();

                            AND EXISTS (
                                SELECT FROM MessagePins WHERE MessagePins.MsgId = Messages.Id
                                AND MessagePins.PinId = #{pin_tag as PinTags::Id}
                            )
                        }
                        Ordering::Greater => {
                            AND MessagePins.PinId = ANY(#{pins as SNOWFLAKE_ARRAY})
                            GROUP BY SelectedMessages./Id, SelectedMessages./PartyId
                            HAVING COUNT(DISTINCT MessagePins.PinId)::int8 = array_length(#{pins as SNOWFLAKE_ARRAY}, 1)
                        },
                    }

                    match cursor {
                        Cursor::After(_)  => { ORDER BY Messages.Id ASC },
                        Cursor::Before(_) => { ORDER BY Messages.Id DESC },
                        _ => {}
                    }

                    LIMIT #{limit as Type::INT2})

                    if *recurse > 0 {
                        UNION ALL (SELECT
                            SelectedMessages.Depth + 1  AS SelectedMessages.Depth,
                            Messages.Id                 AS SelectedMessages.Id,
                            SelectedMessages.PartyId    AS SelectedMessages.PartyId
                        FROM LiveMessages AS Messages INNER JOIN SelectedMessages ON Messages.ParentId = SelectedMessages.Id
                        WHERE SelectedMessages.Depth < #{recurse as Type::INT2}

                        // TODO: Better ordering for children?
                        ORDER BY Messages.Id DESC

                        LIMIT #{limit as Type::INT2})
                    }
                )
            }
        }
        // TODO: DISTINCT ON is a temporary fix
        SELECT DISTINCT ON(SelectedMessages.Id)
            Messages.Id         AS @MsgId,
            Messages.UserId     AS @UserId,
            Messages.RoomId     AS @RoomId,
            Messages.Kind       AS @Kind,
            Messages.ParentId   AS @ParentId,
            Messages.EditedAt   AS @EditedAt,
            Messages.Flags      AS @Flags,

            match req {
                GetMsgRequest::Single { .. } => { FALSE },
                GetMsgRequest::Many { ref user_id, .. } => {
                    (
                        SELECT AggRelationships.RelA = {UserRelationship::BlockedDangerous as i8}
                          FROM AggRelationships
                         WHERE AggRelationships.UserId = Messages.UserId
                           AND AggRelationships.FriendId = #{user_id as Users::Id}
                    ) IS TRUE
                }
            } AS @Unavailable,

            match req {
                GetMsgRequest::Many { ref user_id, .. } => { EXISTS(
                    SELECT FROM MessageStars
                    WHERE MessageStars.MsgId = Messages.Id
                    AND MessageStars.UserId = #{user_id as Users::Id}
                ) }
                _ => { FALSE },
            } AS @Starred,

            SelectedMessages.PartyId        AS @PartyId,
            PartyMembers.JoinedAt           AS @JoinedAt,
            Users.Username                  AS @Username,
            Users.Discriminator             AS @Discriminator,
            Users.Flags                     AS @UserFlags,
            .combine_profile_bits(BaseProfile.Bits, PartyProfile.Bits, PartyProfile.AvatarId) AS @ProfileBits,
            COALESCE(PartyProfile.AvatarId, BaseProfile.AvatarId) AS @AvatarId,
            COALESCE(PartyProfile.Nickname, BaseProfile.Nickname) AS @Nickname,
            Messages.Content                AS @Content,
            AggMentions.Kinds               AS @MentionKinds,
            AggMentions.Ids                 AS @MentionIds,
            (
                SELECT COALESCE(ARRAY_AGG(RoleMembers.RoleId), "{}")
                FROM RoleMembers INNER JOIN Roles ON Roles.Id = RoleMembers.RoleId
                WHERE RoleMembers.UserId = Messages.UserId
                  AND Roles.PartyId = SelectedMessages.PartyId
            ) AS @RoleIds,
            TempAttachments.Meta    AS @AttachmentsMeta,
            TempAttachments.Preview AS @AttachmentsPreviews,

            (
                WITH SortedEmbeds AS (
                    SELECT
                        MessageEmbeds.EmbedId AS SortedEmbeds.EmbedId,
                        MessageEmbeds.Flags AS SortedEmbeds.Flags
                    FROM MessageEmbeds
                    WHERE MessageEmbeds.MsgId = SelectedMessages.Id
                    ORDER BY MessageEmbeds.Position ASC
                )
                SELECT jsonb_agg(jsonb_build_object(
                    "f", SortedEmbeds.Flags,
                    "e", Embeds.Embed
                )) FROM SortedEmbeds INNER JOIN Embeds ON Embeds.Id = SortedEmbeds.EmbedId
            ) AS @Embeds,

            (
                SELECT jsonb_agg(jsonb_build_object(
                    "e", AggReactions.EmoteId,
                    "j", AggReactions.EmojiId,
                    // single queries cannot have own-reaction data
                    "m", match req {
                        GetMsgRequest::Single { .. } => { FALSE },
                        _ => { ReactionUsers.UserId IS NOT NULL }
                    },
                    "c", AggReactions.Count
                )) FROM AggReactions

                // where a user_id is available, check for own reaction in ReactionUsers
                if let GetMsgRequest::Many { ref user_id, .. } = req {
                    LEFT JOIN ReactionUsers ON
                        ReactionUsers.ReactionId = AggReactions.Id
                        AND ReactionUsers.UserId = #{user_id as Users::Id}
                }

                WHERE AggReactions.MsgId = SelectedMessages.Id
            ) AS @Reactions,

            (
                SELECT ARRAY_AGG(MessagePins.PinId)
                FROM MessagePins WHERE MessagePins.MsgId = SelectedMessages.Id
            ) AS @Pins

        FROM SelectedMessages INNER JOIN Messages ON Messages.Id = SelectedMessages.Id
            INNER JOIN Users ON Users.Id = Messages.UserId
            LEFT JOIN Profiles AS BaseProfile
                ON BaseProfile.UserId = Messages.UserId AND BaseProfile.PartyId IS NULL
            LEFT JOIN Profiles AS PartyProfile
                ON PartyProfile.UserId = Messages.UserId AND PartyProfile.PartyId = SelectedMessages.PartyId
            // PartyId can be null for non-party room messages
            LEFT JOIN PartyMembers ON PartyMembers.UserId = Messages.UserId
                AND PartyMembers.PartyId IS NOT DISTINCT FROM SelectedMessages.PartyId
            LEFT JOIN AggMentions ON AggMentions.MsgId = SelectedMessages.Id

            LEFT JOIN LATERAL (
                SELECT
                    (jsonb_agg(jsonb_build_object(
                        "id", Files.Id,
                        "size", Files.Size,
                        "flags", Files.Flags,
                        "name", Files.Name,
                        "mime", Files.Mime,
                        "width", Files.Width,
                        "height", Files.Height
                    ))) AS TempAttachments.Meta,
                    ARRAY_AGG(Files.Preview) AS TempAttachments.Preview
                FROM Attachments INNER JOIN Files ON Files.Id = Attachments.FileId
                WHERE Attachments.MsgId = SelectedMessages.Id
            ) AS TempAttachments ON TRUE
    }).await?;

    let mut last_author: Option<PartyMember> = None;

    Ok(stream.map(move |row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => {
            let party_id: PartyId = row.party_id()?;
            let msg_id: MessageId = row.msg_id()?;

            // many fields here are empty, easy to construct, and are filled in below
            let mut msg = Message {
                id: msg_id,
                party_id,
                room_id: row.room_id()?,
                flags: MessageFlags::empty(),
                kind: MessageKind::Normal,
                edited_at: None,
                content: None,
                author: PartyMember {
                    user: User {
                        id: UserId::null(),
                        discriminator: 0,
                        username: SmolStr::new_inline("SYSTEM"),
                        flags: UserFlags::SYSTEM_USER,
                        presence: None,
                        profile: Nullable::Undefined,
                        email: None,
                        preferences: None,
                    },
                    joined_at: None,
                    flags: PartyMemberFlags::empty(),
                    roles: ThinVec::new(),
                },
                parent: row.parent_id()?,
                user_mentions: ThinVec::new(),
                role_mentions: ThinVec::new(),
                room_mentions: ThinVec::new(),
                attachments: ThinVec::new(),
                reactions: ThinVec::new(),
                embeds: ThinVec::new(),
                pins: ThinVec::new(),
                score: 0,
            };

            // before we continue, if the message was marked unavailable, then we can skip everything else
            if row.unavailable()? {
                msg.kind = MessageKind::Unavailable;

                return Ok(msg);
            }

            msg.kind = MessageKind::try_from(row.kind::<i16>()?).unwrap_or_default();
            msg.flags = MessageFlags::from_bits_truncate_public(row.flags()?);

            msg.author = {
                let id = row.user_id()?;

                match last_author {
                    Some(ref last_user) if last_user.id == id => last_user.clone(),
                    _ => {
                        let user = User {
                            id,
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
                        };

                        let author = PartyMember {
                            user,
                            roles: row.role_ids()?,
                            joined_at: row.joined_at()?,
                            flags: PartyMemberFlags::empty(),
                        };

                        last_author = Some(author.clone());

                        author
                    }
                }
            };

            if msg.flags.contains(MessageFlags::DELETED) {
                msg.flags &= MessageFlags::DELETED | MessageFlags::REMOVED;

                return Ok(msg);
            }

            msg.content = row.content()?;
            msg.edited_at = row.edited_at()?;

            msg.attachments = {
                let mut attachments = ThinVec::new();

                let meta: Option<Json<Vec<schema::AggAttachmentsMeta>>> = row.attachments_meta()?;

                if let Some(Json(meta)) = meta {
                    let previews: Vec<Option<&[u8]>> = row.attachments_previews()?;

                    if meta.len() != previews.len() {
                        return Err(Error::InternalErrorStatic("Meta != Previews length"));
                    }

                    attachments.reserve(meta.len());

                    for (meta, preview) in meta.into_iter().zip(previews) {
                        // NOTE: This filtering is done in the application layer because it
                        // produces sub-optimal query-plans in Postgres.
                        //
                        // Perhaps more intelligent indexes could solve that later.
                        if let Some(raw_flags) = meta.flags {
                            if AttachmentFlags::from_bits_truncate(raw_flags).contains(AttachmentFlags::ORPHANED) {
                                continue; // skip
                            }
                        }

                        attachments.push(Attachment {
                            file: File {
                                id: meta.id,
                                filename: meta.name,
                                size: meta.size as i64,
                                mime: meta.mime,
                                width: meta.width,
                                height: meta.height,
                                preview: preview.and_then(|p| {
                                    use z85::ToZ85;

                                    let mut out = ThinString::with_capacity(p.estimate_z85_encoded_size());
                                    p.to_z85_in(&mut out).ok()?;
                                    Some(out)
                                }),
                            },
                        })
                    }
                }

                attachments
            };

            msg.pins = row.pins::<Option<_>>()?.unwrap_or_default(); // will be NULL if empty

            if row.starred()? {
                msg.flags |= MessageFlags::STARRED;
            }

            match row.reactions()? {
                Some(Json::<Vec<RawReaction>>(raw)) if !raw.is_empty() => {
                    let mut reactions = ThinVec::with_capacity(raw.len());

                    for r in raw {
                        if r.c == 0 {
                            continue;
                        }

                        reactions.push(Reaction::Shorthand(ReactionShorthand {
                            me: r.m,
                            count: r.c,
                            emote: match (r.e, r.j) {
                                (Some(emote), None) => EmoteOrEmoji::Emote { emote },
                                (None, Some(id)) => match state.emoji.id_to_emoji(id) {
                                    Some(emoji) => EmoteOrEmoji::Emoji { emoji },
                                    None => {
                                        log::warn!("Emoji not found for id {id} -- skipping");

                                        continue;
                                    }
                                },
                                _ => {
                                    log::error!("Invalid state for reactions on message {}", msg_id);

                                    continue; // just skip the invalid one
                                }
                            },
                        }));
                    }

                    msg.reactions = reactions;
                }
                _ => {}
            }

            let mention_kinds: Option<Vec<i32>> = row.mention_kinds()?;
            if let Some(mention_kinds) = mention_kinds {
                // lazily parse ids
                let mention_ids: Vec<Snowflake> = row.mention_ids()?;

                if mention_ids.len() != mention_kinds.len() {
                    return Err(Error::InternalErrorStatic("Mismatched Mention aggregates!"));
                }

                for (kind, id) in mention_kinds.into_iter().zip(mention_ids) {
                    let mentions = match kind {
                        1 => &mut msg.user_mentions,
                        2 => &mut msg.role_mentions,
                        3 => &mut msg.room_mentions,
                        _ => unreachable!(),
                    };

                    mentions.push(id);
                }
            }

            if let Some(Json::<Vec<RawEmbed>>(embeds)) = row.embeds()? {
                msg.embeds = embeds
                    .into_iter()
                    .map(|RawEmbed { mut e, f }| {
                        if let (Embed::V1(ref mut v1), Some(f)) = (&mut e, f) {
                            v1.flags |= f;
                        }

                        e
                    })
                    .collect();
            }

            Ok(msg)
        }
    }))
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
struct RawReaction {
    /// emote_id
    pub e: Option<EmoteId>,
    /// emoji_id
    pub j: Option<i32>,
    /// me (own reaction)
    pub m: bool,
    /// count
    pub c: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct RawEmbed {
    /// flags
    #[serde(default)]
    pub f: Option<EmbedFlags>,

    /// embed
    pub e: Embed,
}
