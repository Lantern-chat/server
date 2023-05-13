use std::{collections::BTreeSet, sync::atomic::AtomicUsize};

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

pub async fn get_search(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
    terms: SearchTerms,
) -> Result<SearchResult<impl Stream<Item = Result<Message, Error>>>, Error> {
    let db = state.db.read.get().await?;

    // TODO: Maybe check for room perms in cache if the query is limited to a room?
    let SearchResult { lower_bound, stream } = do_search(
        state,
        &*db,
        100,
        SearchRequest::Search {
            user_id: auth.user_id,
            scope: SearchScope::Party(party_id),
            count: true,
            terms,
        },
    )
    .await?;

    let mut stream = stream.peekable();

    // force the first read so lower_bound is populated
    let _ = std::pin::Pin::new(&mut stream).peek().await;

    Ok(SearchResult { lower_bound, stream })
}

use sdk::api::commands::room::GetMessagesQuery;

fn form_to_search(
    user_id: Snowflake,
    room_id: Snowflake,
    form: GetMessagesQuery,
    needs_perms: bool,
) -> SearchRequest {
    let cursor = form.query.unwrap_or_else(|| Cursor::Before(Snowflake::max_value()));

    // we have to handle multiple pins as a search, so convert all the
    // parameters to search terms
    if form.pinned.len() > 1 {
        let mut terms = SearchTerms::default();

        for pin in form.pinned {
            terms.insert(SearchTerm::new(SearchTermKind::Pinned(pin)));
        }

        terms.insert(SearchTerm::new(match cursor {
            Cursor::After(id) => SearchTermKind::After(id),
            Cursor::Before(id) => SearchTermKind::Before(id),
            Cursor::Exact(id) => SearchTermKind::Id(id),
        }));

        if let Cursor::After(_) = cursor {
            terms.insert(SearchTerm::new(SearchTermKind::Ascending));
        }

        if form.starred {
            terms.insert(SearchTerm::new(SearchTermKind::IsStarred));
        }

        terms.insert(SearchTerm::new(SearchTermKind::Room(room_id)));

        if let Some(parent) = form.parent {
            terms.insert(SearchTerm::new(SearchTermKind::Parent(parent)));
        }

        return SearchRequest::Search {
            user_id,
            scope: SearchScope::Room(room_id),
            count: false,
            terms,
        };
    }

    SearchRequest::Many {
        cursor,
        parent: form.parent,
        user_id,
        room_id,
        needs_perms,
        starred: form.starred,
        pinned: form.pinned.first().copied(),
    }
}

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

    let limit = match form.limit {
        Some(limit) => 100.min(limit as i16),
        None => 100,
    };

    let search = form_to_search(auth.user_id, room_id, form, needs_perms);

    let db = state.db.read.get().await?;

    let SearchResult { stream, .. } = do_search(state, &*db, limit, search).await?;

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

#[derive(Clone, Copy)]
pub enum SearchScope {
    Party(Snowflake),
    Room(Snowflake),
}

pub type SearchTerms = BTreeSet<SearchTerm>;

pub enum SearchRequest {
    Single {
        msg_id: Snowflake,
    },
    Many {
        cursor: Cursor,
        parent: Option<Snowflake>,
        user_id: Snowflake,
        room_id: Snowflake,
        needs_perms: bool,
        starred: bool,
        pinned: Option<Snowflake>,
    },
    Search {
        user_id: Snowflake,
        scope: SearchScope,
        count: bool,
        terms: SearchTerms,
    },
}

impl SearchRequest {
    fn user_id(&self) -> Option<&Snowflake> {
        match self {
            SearchRequest::Many { user_id, .. } | SearchRequest::Search { user_id, .. } => Some(user_id),
            _ => None,
        }
    }
}

pub struct SearchResult<S> {
    pub lower_bound: Arc<AtomicUsize>,
    pub stream: S,
}

pub async fn do_search<DB>(
    state: ServerState,
    db: &DB,
    limit: i16,
    mut search: SearchRequest,
) -> Result<SearchResult<impl Stream<Item = Result<Message, Error>>>, Error>
where
    DB: db::pool::AnyClient,
{
    let data = match search {
        SearchRequest::Search {
            ref mut scope,
            ref mut terms,
            ..
        } => Some(process_terms(terms, scope)),
        _ => None,
    };

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
            }

            struct MessageCount {
                Count: Type::INT8,
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
                        FALSE AS SelectedMessages.Starred
                    FROM Messages INNER JOIN Rooms ON Rooms.Id = Messages.RoomId
                    WHERE Messages.Id = #{msg_id as Messages::Id}
                    AND Messages.Flags & {MessageFlags::DELETED.bits()} = 0
                )
            }
            SearchRequest::Many {
                ref cursor,
                ref parent,
                ref user_id,
                ref room_id,
                ref pinned,
                starred,
                needs_perms,
            } => {
                SelectedMessages AS MATERIALIZED (
                    SELECT
                        Messages.Id AS SelectedMessages.Id,
                        Rooms.PartyId AS SelectedMessages.PartyId,
                        EXISTS(
                            SELECT FROM MessageStars
                            WHERE MessageStars.MsgId = Messages.Id
                            AND MessageStars.UserId = #{user_id as Users::Id}
                        ) AS SelectedMessages.Starred

                    FROM Messages INNER JOIN Rooms ON Rooms.Id = Messages.RoomId

                    if needs_perms {
                        INNER JOIN AggRoomPerms
                            ON AggRoomPerms.RoomId = Messages.RoomId
                            AND AggRoomPerms.UserId = #{user_id as Users::Id}
                    }

                    WHERE
                        Messages.Flags & {MessageFlags::DELETED.bits()} = 0
                    AND Messages.RoomId = #{room_id as Rooms::Id}

                    if let Some(ref parent) = parent {
                        AND Messages.ThreadId = #{parent as Messages::ThreadId}
                    }

                    if starred {
                        AND EXISTS (
                            SELECT FROM MessageStars WHERE MessageStars.MsgId = Messages.Id
                            AND MessageStars.UserId = #{user_id as Users::Id}
                        )
                    }

                    if let Some(ref pin_id) = pinned {
                        AND EXISTS (
                            SELECT FROM MessagePins WHERE MessagePins.MsgId = Messages.Id
                            AND MessagePins.PinId = #{pin_id as PinTags::Id}
                        )
                    }

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
            SearchRequest::Search { ref user_id, ref scope, ref terms, count } => {
                AllowedRooms AS (
                    SELECT AggRoomPerms.RoomId AS AllowedRooms.RoomId
                    FROM   AggRoomPerms
                    WHERE  AggRoomPerms.UserId = #{user_id as Users::Id}
                    AND (
                        // we know this perm is in the lower half, so only use that
                        let perms = Permissions::READ_MESSAGE_HISTORY.to_i64();
                        assert_eq!(perms[1], 0);

                        AggRoomPerms.Permissions1 & {perms[0]} = {perms[0]}
                    )

                    AND match scope {
                        SearchScope::Party(ref party_id) => { AggRoomPerms.PartyId = #{party_id as Party::Id} },
                        SearchScope::Room(ref room_id) => { AggRoomPerms.RoomId = #{room_id as Rooms::Id} }
                    }
                ),
                SelectedMessages AS (
                    let data = data.as_ref().unwrap();

                    let has_many_pins = data.pin_tags.len() > 1;
                    let has_media_query = !data.has_media.is_empty() || !data.has_not_media.is_empty();
                    let has_embed_query = data.has_embed.is_some() || has_media_query;
                    let has_attachment_query = data.has_file.is_some() || has_media_query;

                    let has_difficult_joins = has_embed_query || has_attachment_query || data.has_link;

                    SELECT
                        Messages.Id AS SelectedMessages.Id,
                        Rooms.PartyId AS SelectedMessages.PartyId,
                        // optimize either branch
                        match data.starred {
                            false => {
                                EXISTS(
                                    SELECT FROM MessageStars
                                    WHERE MessageStars.MsgId = Messages.Id
                                    AND MessageStars.UserId = #{user_id as Users::Id}
                                )
                            },
                            // if one of the criteria is to be starred, this will always be true
                            true => { TRUE }
                        } AS SelectedMessages.Starred

                    FROM Messages
                        INNER JOIN Rooms ON Rooms.Id = Messages.RoomId
                        INNER JOIN AllowedRooms ON AllowedRooms.RoomId = Messages.RoomId

                    if has_many_pins {
                        INNER JOIN MessagePins ON MessagePins.MsgId = Messages.Id
                    }

                    WHERE Messages.Flags & {MessageFlags::DELETED.bits()} = 0

                    if has_many_pins {
                        AND MessagePins.PinId = ANY(#{&data.pin_tags as SNOWFLAKE_ARRAY})
                    }

                    for term in terms {
                        AND if term.negated { NOT }
                        (match term.kind {
                            SearchTermKind::Query(ref q)   => { Messages.Ts @@ websearch_to_tsquery(#{q as Type::TEXT}) },
                            SearchTermKind::Id(ref id)     => { Messages.Id = #{id as Messages::Id} },
                            SearchTermKind::Before(ref ts) => { Messages.Id < #{ts as Messages::Id} },
                            SearchTermKind::After(ref ts)  => { Messages.Id > #{ts as Messages::Id} },
                            SearchTermKind::User(ref id)   => { Messages.UserId = #{id as Messages::UserId} },
                            SearchTermKind::Room(ref id)   => { Messages.RoomId = #{id as Messages::RoomId} },
                            SearchTermKind::Parent(ref id) => { Messages.ThreadId = #{id as Messages::ThreadId} },
                            SearchTermKind::InThread       => { Messages.ThreadId IS NOT NULL },
                            SearchTermKind::Has(Has::Link) => { Messages.Flags & {MessageFlags::HAS_LINK.bits()} != 0 },
                            SearchTermKind::IsPinned => {
                                // redundant if selecting by specific tag
                                if !data.pin_tags.is_empty() {
                                    EXISTS ( SELECT FROM MessagePins WHERE MessagePins.MsgId = Messages.Id )
                                } else { TRUE }
                            }
                            _ => { {unreachable!("Unaccounted for search term")} }
                        })
                    }

                    if data.starred {
                        AND EXISTS (
                            SELECT FROM MessageStars
                            WHERE MessageStars.MsgId = Messages.Id
                            AND MessageStars.UserId = #{user_id as Users::Id}
                        )
                    }

                    if data.pin_tags.len() == 1 {
                        let pin_tag = data.pin_tags.first().unwrap();

                        AND EXISTS (
                            SELECT FROM MessagePins WHERE MessagePins.MsgId = Messages.Id
                            AND MessagePins.PinId = #{pin_tag as PinTags::Id}
                        )
                    }

                    if !data.pin_not_tags.is_empty() {
                        AND NOT EXISTS (
                            SELECT FROM MessagePins WHERE MessagePins.MsgId = Messages.Id
                            AND MessagePins.PinId = ANY(#{&data.pin_not_tags as SNOWFLAKE_ARRAY})
                        )
                    }

                    match data.has_embed {
                        Some(false) => {
                            AND NOT EXISTS(
                                SELECT FROM Embeds INNER JOIN MessageEmbeds ON
                                MessageEmbeds.MsgId = Messages.Id AND Embeds.Id = MessageEmbeds.EmbedId
                            )
                        }
                        Some(true) if !has_media_query => {
                            AND EXISTS(
                                SELECT FROM Embeds INNER JOIN MessageEmbeds ON
                                MessageEmbeds.MsgId = Messages.Id AND Embeds.Id = MessageEmbeds.EmbedId
                            )
                        }
                        _ => {
                            if !data.has_media.is_empty() {
                                AND EXISTS(
                                    SELECT FROM Embeds INNER JOIN MessageEmbeds ON
                                    MessageEmbeds.MsgId = Messages.Id AND Embeds.Id = MessageEmbeds.EmbedId
                                    WHERE Embeds.Embed->>"ty" = ALL(ARRAY[
                                        join has in &data.has_media { {has.as_str()} }
                                    ])
                                )
                            }

                            if !data.has_not_media.is_empty() {
                                AND NOT EXISTS(
                                    SELECT FROM Embeds INNER JOIN MessageEmbeds ON
                                    MessageEmbeds.MsgId = Messages.Id AND Embeds.Id = MessageEmbeds.EmbedId
                                    WHERE Embeds.Embed->>"ty" = ANY(ARRAY[
                                        join has in &data.has_not_media { {has.as_str()} }
                                    ])
                                )
                            }
                        }
                    }

                    match data.has_file {
                        Some(false) => {
                            AND NOT EXISTS(
                                SELECT FROM Files INNER JOIN Attachments ON
                                Attachments.MessageId = Messages.Id AND Files.Id = Attachments.FileId
                            )
                        }
                        Some(true) if !has_media_query => {
                            AND EXISTS(
                                SELECT FROM Files INNER JOIN Attachments ON
                                Attachments.MessageId = Messages.Id AND Files.Id = Attachments.FileId
                            )
                        }
                        _ => {
                            if !data.has_media.is_empty() {
                                AND EXISTS(
                                    SELECT FROM Files INNER JOIN Attachments ON
                                    Attachments.MessageId = Messages.Id AND Files.Id = Attachments.FileId
                                    WHERE TRUE for has in &data.has_media {
                                        AND starts_with(Files.Mime, { has.as_mime() })
                                    }
                                )
                            }

                            if !data.has_not_media.is_empty() {
                                AND NOT EXISTS(
                                    SELECT FROM Files INNER JOIN Attachments ON
                                    Attachments.MessageId = Messages.Id AND Files.Id = Attachments.FileId
                                    WHERE FALSE for has in &data.has_not_media {
                                        OR starts_with(Files.Mime, { has.as_mime() })
                                    }
                                )
                            }
                        }
                    }

                    if has_many_pins {
                        GROUP BY Messages.Id, Rooms.PartyId
                        HAVING COUNT(DISTINCT MessagePins.PinId)::int8 = {data.pin_tags.len() as i64}
                    }

                    match data.ascending {
                        true => { ORDER BY Messages.Id ASC },
                        false => { ORDER BY Messages.Id DESC },
                    }

                    if !count || has_difficult_joins {
                        LIMIT {limit + 1}
                    }
                )
            }
        }, MessageCount AS MATERIALIZED (
            SELECT COUNT(*)::int8 AS MessageCount.Count FROM SelectedMessages
        )
        SELECT
            Messages.Id         AS @MsgId,
            Messages.UserId     AS @UserId,
            Messages.RoomId     AS @RoomId,
            Messages.Kind       AS @Kind,
            Messages.ThreadId   AS @ThreadId,
            Messages.EditedAt   AS @EditedAt,
            Messages.Flags      AS @Flags,

            if let SearchRequest::Single { .. } = search { FALSE } else {
                // RelA could be NULL, so use IS TRUE
                (AggRelationships.RelA = {UserRelationship::BlockedDangerous as i8}) IS TRUE
            } AS @Unavailable,

            SelectedMessages.Starred        AS @Starred,
            SelectedMessages.PartyId        AS @PartyId,
            AggMembers.JoinedAt AS @JoinedAt,
            Users.Username      AS @Username,
            Users.Discriminator AS @Discriminator,
            Users.Flags         AS @UserFlags,
            MessageCount.Count  AS @Count,
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
                if let Some(user_id) = search.user_id() {
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

            if let Some(user_id) = search.user_id() {
                LEFT JOIN AggRelationships
                    ON AggRelationships.UserId = Messages.UserId
                    AND AggRelationships.FriendId = #{user_id as Users::Id}
            }

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

        LIMIT {limit}
    }).await?;

    let mut last_user: Option<User> = None;
    let count = Arc::new(AtomicUsize::new(0));

    Ok(SearchResult {
        lower_bound: count.clone(),
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

                count.store(row.count::<i64>()? as usize, std::sync::atomic::Ordering::Relaxed);

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

#[derive(Default)]
pub struct ProcessedSearch {
    has_media: ArrayVec<Has, 3>,
    has_not_media: ArrayVec<Has, 3>,
    has_embed: Option<bool>,
    has_file: Option<bool>,
    pin_tags: Vec<Snowflake>,
    pin_not_tags: Vec<Snowflake>,
    has_link: bool,
    starred: bool,
    ascending: bool,
}

fn process_terms(terms: &mut SearchTerms, scope: &mut SearchScope) -> ProcessedSearch {
    let mut data = ProcessedSearch::default();

    #[allow(clippy::match_like_matches_macro)]
    terms.retain(|term| match term.kind {
        SearchTermKind::Query(_) => true,
        SearchTermKind::Before(_) => true,
        SearchTermKind::After(_) => true,
        SearchTermKind::Ascending => {
            data.ascending = true;
            false
        }
        SearchTermKind::User(_) => true,
        SearchTermKind::Room(id) => {
            // since we limit to rooms anyway
            *scope = SearchScope::Room(id);
            false
        }
        SearchTermKind::InThread => true,
        SearchTermKind::IsStarred => {
            data.starred = true;
            true
        }
        SearchTermKind::IsPinned => true,
        SearchTermKind::Pinned(tag) => {
            if term.negated {
                data.pin_not_tags.push(tag);
            } else {
                data.pin_tags.push(tag);
            }
            false
        }
        SearchTermKind::Has(has) => {
            match has {
                Has::Image | Has::Video | Has::Audio => {
                    if term.negated {
                        data.has_not_media.push(has);
                    } else {
                        data.has_media.push(has);
                    }
                }
                Has::Embed => data.has_embed = Some(!term.negated),
                Has::File => data.has_file = Some(!term.negated),
                Has::Link => {
                    data.has_link = true;
                    return true;
                }
            }

            false
        }
        _ => false,
    });

    data
}
