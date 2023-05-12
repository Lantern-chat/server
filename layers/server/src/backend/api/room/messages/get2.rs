use std::collections::BTreeSet;

use arrayvec::ArrayVec;
use db::{
    pg::Statement,
    pool::{Client, Object, Transaction},
};
use futures::{FutureExt, Stream, StreamExt};

use schema::{
    flags::AttachmentFlags,
    search::{Has, SearchTerm, SearchTermKind},
    Snowflake, SnowflakeExt,
};
use sdk::models::*;
use smallvec::SmallVec;
use thorn::pg::{Json, ToSql};

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

use sdk::api::commands::room::GetMessagesQuery;

pub async fn get_many(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    form: GetMessagesQuery,
) -> Result<impl Stream<Item = Result<Message, Error>>, Error> {
    let needs_perms = match state.perm_cache.get(auth.user_id, room_id).await {
        Some(perms) => {
            if !perms.contains(Permissions::READ_MESSAGE_HISTORY) {
                return Err(Error::NotFound);
            }

            false
        }
        None => true,
    };

    let cursor = form.query.unwrap_or_else(|| Cursor::Before(Snowflake::max_value()));
    let limit = match form.limit {
        Some(limit) => 100.min(limit as i16),
        None => 100,
    };

    let db = state.db.read.get().await?;

    let SearchResult { stream, .. } = do_search(
        state,
        &*db,
        limit,
        SearchRequest::Many {
            cursor,
            user_id: auth.user_id,
            room_id,
            needs_perms,
        },
    )
    .await?;

    Ok(stream)
}

pub async fn get_one<DB>(state: ServerState, db: &DB, msg_id: Snowflake) -> Result<Message, Error>
where
    DB: db::pool::AnyClient,
{
    let SearchResult { stream, .. } = do_search(state, db, 1, SearchRequest::Single { msg_id }).await?;

    let mut stream = std::pin::pin!(stream);

    match stream.next().await {
        Some(Ok(msg)) => Ok(msg),
        Some(Err(e)) => Err(e),
        None => Err(Error::NotFound),
    }
}

pub enum SearchRequest {
    Single {
        msg_id: Snowflake,
    },
    Many {
        cursor: Cursor,
        user_id: Snowflake,
        room_id: Snowflake,
        needs_perms: bool,
    },
    Search {
        user_id: Snowflake,
        party_id: Snowflake,
        terms: BTreeSet<SearchTerm>,
    },
}

pub struct SearchResult<S> {
    pub upper_bound: Option<usize>,
    pub stream: S,
}

pub async fn do_search<DB>(
    state: ServerState,
    db: &DB,
    limit: i16,
    search: SearchRequest,
) -> Result<SearchResult<impl Stream<Item = Result<Message, Error>>>, Error>
where
    DB: db::pool::AnyClient,
{
    #[rustfmt::skip]
    let stream = db.query_stream2(schema::sql! {
        tables! {
            struct AllowedRooms {
                RoomId: SNOWFLAKE_ARRAY,
            }

            struct SelectedMessages {
                Id: Messages::Id,
                PartyId: Party::Id,
                Starred: Type::BOOL,
                Unavailable: Type::BOOL,
            }

            struct Starred {
                Starred: Type::BOOL,
            }

            pub struct SortedEmbeds {
                EmbedId: MessageEmbeds::EmbedId,
                Flags: MessageEmbeds::Flags,
            }

            pub struct TempAttachments {
                Meta: Type::JSONB,
                Preview: Type::BYTEA_ARRAY,
            }
        };

        WITH

        match search {
            SearchRequest::Single { ref msg_id } => {
                SelectedMessages AS (
                    SELECT
                        Messages.Id AS SelectedMessages.Id,
                        Rooms.PartyId AS SelectedMessages.PartyId,
                        FALSE AS SelectedMessages.Starred,
                        FALSE AS SelectedMessages.Unavailable
                    FROM Messages INNER JOIN Rooms ON Rooms.Id = Messages.RoomId
                    WHERE Messages.Id = #{msg_id as Messages::Id}
                    AND Messages.Flags & {MessageFlags::DELETED.bits()} = 0
                )
            }
            SearchRequest::Many { ref cursor, ref user_id, ref room_id, needs_perms } => {
                SelectedMessages AS MATERIALIZED (
                    SELECT
                        Messages.Id AS SelectedMessages.Id,
                        Rooms.PartyId AS SelectedMessages.PartyId,
                        EXISTS(
                            SELECT FROM MessageStars
                            WHERE MessageStars.MsgId = Messages.Id
                            AND MessageStars.UserId = #{user_id as Users::Id}
                        ) AS SelectedMessages.Starred,

                        // RelA could be NULL, so use IS TRUE
                        (AggRelationships.RelA = {UserRelationship::BlockedDangerous as i8}) IS TRUE
                            AS SelectedMessages.Unavailable

                    FROM Messages INNER JOIN Rooms ON Rooms.Id = Messages.RoomId

                    if needs_perms {
                        INNER JOIN AggRoomPerms
                            ON AggRoomPerms.RoomId = Messages.RoomId
                            AND AggRoomPerms.UserId = #{user_id as Users::Id}
                    }

                    LEFT JOIN AggRelationships
                        ON AggRelationships.UserId = Messages.UserId
                        AND AggRelationships.FriendId = #{user_id as Users::Id}

                    WHERE
                        Messages.Flags & {MessageFlags::DELETED.bits()} = 0
                    AND Messages.RoomId = #{room_id as Rooms::Id}

                    if needs_perms {
                        // we know this perm is in the lower half, so only use that
                        let perms = Permissions::READ_MESSAGE_HISTORY.to_i64();
                        assert_eq!(perms[1], 0);

                        AND AggRoomPerms.Permissions1 & {perms[0]} = {perms[0]}
                    }

                    // this must go last, as it includes ORDER BY
                    AND match cursor {
                        Cursor::After(ref msg_id)  => { Messages.Id > #{msg_id as Messages::Id} ORDER BY Messages.Id ASC },
                        Cursor::Before(ref msg_id) => { Messages.Id < #{msg_id as Messages::Id} ORDER BY Messages.Id DESC },
                        Cursor::Exact(ref msg_id)  => { Messages.Id = #{msg_id as Messages::Id} }
                    }

                    LIMIT {limit}
                )
            },
            SearchRequest::Search {..} => {}
        }
        SELECT
            Messages.Id         AS @MsgId,
            Messages.UserId     AS @UserId,
            Messages.RoomId     AS @RoomId,
            Messages.Kind       AS @Kind,
            Messages.ThreadId   AS @ThreadId,
            Messages.EditedAt   AS @EditedAt,
            Messages.Flags      AS @Flags,
            SelectedMessages.Unavailable    AS @Unavailable,
            SelectedMessages.Starred        AS @Starred,
            SelectedMessages.PartyId        AS @PartyId,
            AggMembers.JoinedAt AS @JoinedAt,
            Users.Username      AS @Username,
            Users.Discriminator AS @Discriminator,
            Users.Flags         AS @UserFlags,
            .combine_profile_bits(BaseProfile.Bits, PartyProfile.Bits, PartyProfile.AvatarId) AS @ProfileBits,
            COALESCE(PartyProfile.AvatarId, BaseProfile.AvatarId) AS @AvatarId,
            COALESCE(PartyProfile.Nickname, BaseProfile.Nickname) AS @Nickname,
            Messages.Content        AS @Content,
            AggMentions.Kinds       AS @MentionKinds,
            AggMentions.Ids         AS @MentionIds,
            AggMembers.RoleIds      AS @RoleIds,
            TempAttachments.Meta    AS @AttachmentsMeta,
            TempAttachments.Preview AS @AttachmentsPreviews,

            (
                WITH SortedEmbeds AS (
                    SELECT
                        MessageEmbeds.EmbedId AS SortedEmbeds.EmbedId,
                        MessageEmbeds.Flags AS SortedEmbeds.Flags
                    FROM MessageEmbeds
                    WHERE MessageEmbeds.MsgId = Messages.Id
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
                    "m", match search {
                        SearchRequest::Single { .. } => { FALSE },
                        SearchRequest::Many { .. } | SearchRequest::Search { .. } => {
                            ReactionUsers.UserId IS NOT NULL
                        }
                    },
                    "c", AggReactions.Count
                )) FROM AggReactions

                // where a user_id is available, check for own reaction in ReactionUsers
                if let SearchRequest::Many { ref user_id, .. } | SearchRequest::Search { ref user_id, .. } = search {
                    LEFT JOIN ReactionUsers ON
                        ReactionUsers.ReactionId = AggReactions.Id
                        AND ReactionUsers.UserId = #{user_id as Users::Id}
                }

                WHERE AggReactions.MsgId = Messages.Id
            ) AS @Reactions

        FROM Messages INNER JOIN SelectedMessages ON Messages.Id = SelectedMessages.Id
            INNER JOIN Users ON Users.Id = Messages.UserId
            LEFT JOIN Profiles AS BaseProfile  ON  BaseProfile.UserId = Messages.UserId AND BaseProfile.PartyId IS NULL
            LEFT JOIN Profiles AS PartyProfile ON PartyProfile.UserId = Messages.UserId AND PartyProfile.PartyId = SelectedMessages.PartyId
            LEFT JOIN AggMembers ON AggMembers.UserId = Messages.UserId AND
                (AggMembers.PartyId = SelectedMessages.PartyId OR (
                    // not in a party
                    AggMembers.PartyId IS NULL AND SelectedMessages.PartyId IS NULL
                ))
            LEFT JOIN AggMentions ON AggMentions.MsgId = Messages.Id
            // use a lateral join to avoid GROUP BY
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
                WHERE Attachments.MessageId = Messages.Id
            ) AS TempAttachments ON TRUE
    }).await?;

    let mut last_user: Option<User> = None;

    Ok(SearchResult {
        upper_bound: None,
        stream: stream.map(move |row| match row {
            Err(e) => Err(e.into()),
            Ok(row) => {
                let party_id: Option<Snowflake> = row.party_id()?;
                let msg_id: Snowflake = row.msg_id()?;

                // many fields here are empty, easy to construct, and are filled in below
                let mut msg = Message {
                    id: msg_id,
                    party_id,
                    room_id: row.room_id()?,
                    flags: MessageFlags::empty(),
                    kind: MessageKind::Normal,
                    edited_at: None,
                    content: None,
                    author: make_system_user(),
                    member: None,
                    parent: row.thread_id()?,
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

                msg.author = {
                    let id = row.user_id()?;

                    match last_user {
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

                            last_user = Some(user.clone());

                            user
                        }
                    }
                };

                msg.flags = MessageFlags::from_bits_truncate_public(row.flags()?);
                msg.kind = MessageKind::try_from(row.kind::<i16>()?).unwrap_or_default();

                msg.member = match party_id {
                    None => None,
                    Some(_) => Some(PartialPartyMember {
                        roles: row.role_ids()?,
                        joined_at: row.joined_at()?,
                        flags: None,
                    }),
                };

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
                            use z85::ToZ85;

                            // NOTE: This filtering is done in the application layer because it
                            // produces sub-optimal query-plans in Postgres.
                            //
                            // Perhaps more intelligent indexes could solve that later.
                            if let Some(raw_flags) = meta.flags {
                                if AttachmentFlags::from_bits_truncate(raw_flags)
                                    .contains(AttachmentFlags::ORPHANED)
                                {
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
                                    preview: preview.and_then(|p| p.to_z85().ok()),
                                },
                            })
                        }
                    }

                    attachments
                };

                // msg.pins =
                //     row.try_get::<_, Option<ThinVec<Snowflake>>>(DynamicMsgColumns::pin_tags())?.unwrap_or_default();

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
        }),
    })
}

pub const fn make_system_user() -> User {
    User {
        id: Snowflake(unsafe { std::num::NonZeroU64::new_unchecked(1) }),
        discriminator: 0,
        username: SmolStr::new_inline("SYSTEM"),
        flags: UserFlags::SYSTEM_USER,
        presence: None,
        profile: Nullable::Undefined,
        email: None,
        preferences: None,
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
struct RawReaction {
    /// emote_id
    pub e: Option<Snowflake>,
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
