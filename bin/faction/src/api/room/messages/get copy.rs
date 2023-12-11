use std::sync::atomic::AtomicUsize;

use arrayvec::ArrayVec;

use futures::{Stream, StreamExt};

use schema::{
    flags::AttachmentFlags,
    search::{Has, SearchError, SearchTerm, SearchTermKind, SearchTerms, Sort},
    Snowflake, SnowflakeExt,
};
use sdk::models::*;
use thorn::pg::Json;

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, prelude::*};

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
        1000,
        SearchRequest::Search(Box::new(Search {
            auth,
            scope: SearchScope::Party(party_id),
            party_id: Some(party_id),
            count: true,
            terms,
        })),
    )
    .await?;

    let mut stream = stream.peekable();

    // force the first read so lower_bound is populated
    let _ = std::pin::Pin::new(&mut stream).peek().await;

    Ok(SearchResult { lower_bound, stream })
}

use sdk::api::commands::room::GetMessagesQuery;

fn form_to_search(
    auth: Authorization,
    room_id: Snowflake,
    form: GetMessagesQuery,
    needs_perms: bool,
) -> SearchRequest {
    let cursor = form.query.unwrap_or_else(|| Cursor::Before(Snowflake::max_value()));

    let mut terms = SearchTerms::empty();

    for pin in form.pinned {
        terms.insert(SearchTerm::new(SearchTermKind::Pinned(pin)));
    }

    terms.insert(SearchTerm::new(match cursor {
        Cursor::After(id) => SearchTermKind::After(id),
        Cursor::Before(id) => SearchTermKind::Before(id),
        Cursor::Exact(id) => SearchTermKind::Id(id),
    }));

    if let Cursor::After(_) = cursor {
        terms.insert(SearchTerm::new(SearchTermKind::Sort(Sort::Ascending)));
    }

    if form.starred {
        terms.insert(SearchTerm::new(SearchTermKind::IsStarred));
    }

    if let Some(parent) = form.parent {
        terms.insert(SearchTerm::new(SearchTermKind::Parent(parent)));
    }

    SearchRequest::Search(Box::new(Search {
        auth,
        scope: SearchScope::Room(room_id, needs_perms),
        party_id: None,
        count: false,
        terms,
    }))
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

    let search = form_to_search(auth, room_id, form, needs_perms);

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
    Room(Snowflake, bool),
}

pub struct Search {
    pub auth: Authorization,
    pub scope: SearchScope,
    pub party_id: Option<Snowflake>,
    pub count: bool,
    pub terms: SearchTerms,
}

pub enum SearchRequest {
    /// Single unauthorized message
    Single {
        msg_id: Snowflake,
    },
    Search(Box<Search>),
}

impl SearchRequest {
    fn user_id(&self) -> Option<&Snowflake> {
        match self {
            SearchRequest::Search(ref search) => Some(&search.auth.user_id),
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
        SearchRequest::Search(ref mut search) => Some(process_terms(
            &state,
            &search.auth,
            &mut search.terms,
            &mut search.scope,
        )?),
        _ => None,
    };

    #[rustfmt::skip]
    let stream = db.query_stream2(schema::sql! {
        tables! {
            struct AllowedRooms {
                Id: AggRoomPerms::Id,
                PartyId: AggRoomPerms::PartyId,
            }

            struct SortedMessages {
                Id: Messages::Id,
                UserId: Messages::UserId,
                RoomId: Messages::RoomId,
                ParentId: Messages::ParentId,
                UpdatedAt: Messages::UpdatedAt,
                EditedAt: Messages::EditedAt,
                Kind: Messages::Kind,
                Flags: Messages::Flags,
                Content: Messages::Content,
                Ts: Messages::Ts,
                Codes: Messages::Codes,
            }

            struct SelectedMessages {
                Id: SortedMessages::Id,
                UserId: SortedMessages::UserId,
                RoomId: SortedMessages::RoomId,
                ParentId: SortedMessages::ParentId,
                UpdatedAt: SortedMessages::UpdatedAt,
                EditedAt: SortedMessages::EditedAt,
                Kind: SortedMessages::Kind,
                Flags: SortedMessages::Flags,
                Content: SortedMessages::Content,

                PartyId: Party::Id,
                Starred: Type::BOOL,
                Rank: Type::FLOAT4,
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

            pub struct Precalculated {
                Query: Type::TSQUERY,
            }
        };

        WITH

        match search {
            SearchRequest::Single { ref msg_id } => {
                SelectedMessages AS (
                    SELECT
                        Messages.Id AS SelectedMessages.Id,
                        Messages.UserId AS SelectedMessages.UserId,
                        Messages.RoomId AS SelectedMessages.RoomId,
                        Messages.ParentId AS SelectedMessages.ParentId,
                        Messages.UpdatedAt AS SelectedMessages.UpdatedAt,
                        Messages.EditedAt AS SelectedMessages.EditedAt,
                        Messages.Kind AS SelectedMessages.Kind,
                        Messages.Flags AS SelectedMessages.Flags,
                        Messages.Content AS SelectedMessages.Content,
                        LiveRooms.PartyId AS SelectedMessages.PartyId,
                        FALSE AS SelectedMessages.Starred
                    FROM Messages INNER JOIN LiveRooms ON LiveRooms.Id = Messages.RoomId
                    WHERE Messages.Id = #{msg_id as Messages::Id}
                )
            }
            SearchRequest::Search(ref search) => {
                let Search { ref auth, ref scope, ref terms, ref party_id, count } = **search;

                let data = data.as_ref().unwrap();

                if let Some(ref query) = data.query {
                    // `WHERE ts @@ websearch_to_tsquery(...)` will recompute the tsquery every row, which is bad
                    // so we will precalculate the tsquery here and reuse it
                    Precalculated AS MATERIALIZED (
                        let party_id = party_id.as_ref().unwrap();

                        SELECT (websearch_to_tsquery(
                            .to_language((Party.Flags >> 26)::int2),
                            #{query as Type::TEXT}
                        )) AS Precalculated.Query
                        FROM Party WHERE Party.Id = #{party_id as Party::Id}
                    ),
                }
                AllowedRooms AS NOT MATERIALIZED (
                    // fast path for logged-in user
                    if let SearchScope::Room(room_id, false) = scope {
                        SELECT
                            Rooms.Id AS AllowedRooms.Id,
                            Rooms.PartyId AS AllowedRooms.PartyId
                        FROM LiveRooms AS Rooms
                        WHERE Rooms.Id = #{room_id as Rooms::Id}
                    } else {
                        SELECT
                            Rooms.Id AS AllowedRooms.Id,
                            Rooms.PartyId AS AllowedRooms.PartyId
                        FROM AggRoomPerms AS Rooms
                        WHERE match scope {
                            SearchScope::Party(party_id) => { Rooms.PartyId = #{party_id as Rooms::PartyId} }
                            SearchScope::Room(room_id, _) => { Rooms.Id = #{room_id as Rooms::Id} }
                        }

                        // we know this perm is in the lower half, so only use that
                        let perms = Permissions::READ_MESSAGE_HISTORY.to_i64();
                        assert_eq!(perms[1], 0);

                        AND Rooms.UserId = #{auth.user_id_ref() as Users::Id}
                        AND Rooms.Permissions1 & {perms[0]} = {perms[0]}
                    }
                ),
                SortedMessages AS NOT MATERIALIZED (
                    SELECT
                        Messages.Id AS SortedMessages.Id,
                        Messages.UserId AS SortedMessages.UserId,
                        Messages.RoomId AS SortedMessages.RoomId,
                        Messages.ParentId AS SortedMessages.ParentId,
                        Messages.UpdatedAt AS SortedMessages.UpdatedAt,
                        Messages.EditedAt AS SortedMessages.EditedAt,
                        Messages.Kind AS SortedMessages.Kind,
                        Messages.Flags AS SortedMessages.Flags,
                        Messages.Content AS SortedMessages.Content,
                        Messages.Ts AS SortedMessages.Ts,
                        Messages.Codes AS SortedMessages.Codes
                    FROM Messages

                    if !data.prefers_post_sort {
                        match data.order {
                            Sort::Ascending => { ORDER BY Messages.Id ASC },
                            Sort::Descending => { ORDER BY Messages.Id DESC },
                            Sort::Relevant => {}
                        }
                    }
                ),
                SelectedMessages AS NOT MATERIALIZED (
                    let has_many_pins = data.pin_tags.len() > 1;
                    let has_media_query = !data.has_media.is_empty() || !data.has_not_media.is_empty();
                    let has_embed_query = data.has_embed.is_some() || has_media_query;
                    let has_attachment_query = data.has_file.is_some() || has_media_query;

                    let has_difficult_joins = has_embed_query || has_attachment_query || data.has_link;

                    SELECT
                        Messages.Id AS SelectedMessages.Id,
                        Messages.UserId AS SelectedMessages.UserId,
                        Messages.RoomId AS SelectedMessages.RoomId,
                        Messages.ParentId AS SelectedMessages.ParentId,
                        Messages.UpdatedAt AS SelectedMessages.UpdatedAt,
                        Messages.EditedAt AS SelectedMessages.EditedAt,
                        Messages.Kind AS SelectedMessages.Kind,
                        Messages.Flags AS SelectedMessages.Flags,
                        Messages.Content AS SelectedMessages.Content,

                        Rooms.PartyId AS SelectedMessages.PartyId,

                        if data.order == Sort::Relevant && data.query.is_some() {
                            // TODO: Determine best normalization method
                            // https://www.postgresql.org/docs/current/textsearch-controls.html
                            (ts_rank_cd(Messages.Ts, Precalculated.Query, 4)) AS SelectedMessages.Rank,
                        }

                        // optimize either branch
                        match data.starred {
                            false => {
                                EXISTS(
                                    SELECT FROM MessageStars
                                    WHERE MessageStars.MsgId = Messages.Id
                                    AND MessageStars.UserId = #{auth.user_id_ref() as Users::Id}
                                )
                            },
                            // if one of the criteria is to be starred, this will always be true
                            true => { TRUE }
                        } AS SelectedMessages.Starred

                    FROM SortedMessages AS Messages INNER JOIN AllowedRooms AS Rooms
                        ON Rooms.Id = Messages.RoomId

                    if has_many_pins {
                        INNER JOIN MessagePins ON MessagePins.MsgId = Messages.Id
                    }

                    if data.query.is_some() {
                        INNER JOIN Precalculated ON TRUE
                    }

                    WHERE TRUE

                    if has_many_pins {
                        AND MessagePins.PinId = ANY(#{&data.pin_tags as SNOWFLAKE_ARRAY})
                    }

                    for term in terms {
                        AND if term.negated { NOT }
                        (match term.kind {
                            // These three MUST match the `msg_content_idx` value
                            SearchTermKind::Prefix(ref q)  => { lower(Messages.Content) SIMILAR TO #{q as Type::TEXT} },
                            SearchTermKind::Regex(ref re)  => { lower(Messages.Content) ~* #{re as Type::TEXT} },
                            SearchTermKind::Has(Has::Text) => { lower(Messages.Content) != "" AND lower(Messages.Content) IS NOT NULL }
                            SearchTermKind::Has(Has::Code) => { cardinality(Messages.Codes) > 0 }
                            SearchTermKind::Query(_)       => { Messages.Ts @@ Precalculated.Query },
                            SearchTermKind::Id(ref id)     => { Messages.Id = #{id as Messages::Id} },
                            SearchTermKind::Before(ref ts) => { Messages.Id < #{ts as Messages::Id} },
                            SearchTermKind::After(ref ts)  => { Messages.Id > #{ts as Messages::Id} },
                            SearchTermKind::User(ref id)   => { Messages.UserId = #{id as Messages::UserId} },
                            SearchTermKind::Room(ref id)   => { Messages.RoomId = #{id as Messages::RoomId} },
                            SearchTermKind::Parent(ref id) => { Messages.ParentId = #{id as Messages::ParentId} },
                            SearchTermKind::InThread       => { Messages.ParentId IS NOT NULL },
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
                            AND MessageStars.UserId = #{auth.user_id_ref() as Users::Id}
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

                    if (Some(false), Some(false)) == (data.has_embed, data.has_file) {
                        AND NOT EXISTS(
                            SELECT FROM Embeds INNER JOIN MessageEmbeds ON
                            MessageEmbeds.MsgId = Messages.Id AND Embeds.Id = MessageEmbeds.EmbedId
                            UNION ALL
                            SELECT FROM Files INNER JOIN Attachments ON
                            Attachments.MessageId = Messages.Id AND Files.Id = Attachments.FileId
                        )
                    } else if !has_media_query {
                        if data.has_embed == Some(true) {
                            AND EXISTS(
                                SELECT FROM Embeds INNER JOIN MessageEmbeds ON
                                MessageEmbeds.MsgId = Messages.Id AND Embeds.Id = MessageEmbeds.EmbedId
                            )
                        }

                        if data.has_file == Some(true) {
                            AND EXISTS(
                                SELECT FROM Files INNER JOIN Attachments ON
                                Attachments.MessageId = Messages.Id AND Files.Id = Attachments.FileId
                            )
                        }
                    } else {
                        if data.has_embed == Some(false) {
                            AND NOT EXISTS(
                                SELECT FROM Embeds INNER JOIN MessageEmbeds ON
                                MessageEmbeds.MsgId = Messages.Id AND Embeds.Id = MessageEmbeds.EmbedId
                            )
                        }

                        if data.has_file == Some(false) {
                            AND NOT EXISTS(
                                SELECT FROM Files INNER JOIN Attachments ON
                                Attachments.MessageId = Messages.Id AND Files.Id = Attachments.FileId
                            )
                        }

                        if !data.has_media.is_empty() {
                            AND EXISTS(
                                if data.has_embed != Some(false) {
                                    SELECT FROM Embeds INNER JOIN MessageEmbeds ON
                                    MessageEmbeds.MsgId = Messages.Id AND Embeds.Id = MessageEmbeds.EmbedId
                                    WHERE Embeds.Embed->>"ty" = ALL(ARRAY[
                                        join has in &data.has_media { {has.as_str()} }
                                    ])

                                    if data.has_file != Some(false) {
                                        UNION ALL
                                    }
                                }

                                if data.has_file != Some(false) {
                                    SELECT FROM Files INNER JOIN Attachments ON
                                    Attachments.MessageId = Messages.Id AND Files.Id = Attachments.FileId
                                    WHERE TRUE for has in &data.has_media {
                                        AND starts_with(Files.Mime, { has.as_mime() })
                                    }
                                }
                            )
                        }

                        if !data.has_not_media.is_empty() {
                            AND NOT EXISTS(
                                if data.has_embed != Some(false) {
                                    SELECT FROM Embeds INNER JOIN MessageEmbeds ON
                                    MessageEmbeds.MsgId = Messages.Id AND Embeds.Id = MessageEmbeds.EmbedId
                                    WHERE Embeds.Embed->>"ty" = ANY(ARRAY[
                                        join has in &data.has_not_media { {has.as_str()} }
                                    ])

                                    if data.has_file != Some(false) {
                                        UNION ALL
                                    }
                                }

                                if data.has_file != Some(false) {
                                    SELECT FROM Files INNER JOIN Attachments ON
                                    Attachments.MessageId = Messages.Id AND Files.Id = Attachments.FileId
                                    WHERE FALSE for has in &data.has_not_media {
                                        OR starts_with(Files.Mime, { has.as_mime() })
                                    }
                                }
                            )
                        }
                    }

                    if has_many_pins {
                        GROUP BY Messages.Id, Rooms.PartyId
                        HAVING COUNT(DISTINCT MessagePins.PinId)::int8 = array_length(#{&data.pin_tags as SNOWFLAKE_ARRAY}, 1)
                    }

                    if data.prefers_post_sort {
                        match data.order {
                            Sort::Ascending => { ORDER BY Messages.Id ASC },
                            Sort::Descending => { ORDER BY Messages.Id DESC },
                            Sort::Relevant => {
                                // NOTE: Must use column-name syntax since we're _inside_ SelectedMessages
                                ORDER BY SelectedMessages./Rank DESC, Messages.Id DESC
                            }
                        }
                    }

                    if !count || has_difficult_joins {
                        LIMIT {limit + 1}
                    }
                )

                if count {
                    , MessageCount AS MATERIALIZED (
                        SELECT COUNT(SelectedMessages.Id)::int8 AS MessageCount.Count FROM SelectedMessages
                    )
                }
            }
        }
        SELECT
            SelectedMessages.Id         AS @MsgId,
            SelectedMessages.UserId     AS @UserId,
            SelectedMessages.RoomId     AS @RoomId,
            SelectedMessages.Kind       AS @Kind,
            SelectedMessages.ParentId   AS @ParentId,
            SelectedMessages.EditedAt   AS @EditedAt,
            SelectedMessages.Flags      AS @Flags,

            if let SearchRequest::Single { .. } = search { FALSE } else {
                // RelA could be NULL, so use IS TRUE
                (AggRelationships.RelA = {UserRelationship::BlockedDangerous as i8}) IS TRUE
            } AS @Unavailable,

            match search {
                SearchRequest::Search(ref search) if search.count => {
                    (SELECT MessageCount.Count FROM MessageCount)
                },
                _ => { 0::int8 }
            } AS @Count,

            SelectedMessages.Starred        AS @Starred,
            SelectedMessages.PartyId        AS @PartyId,
            PartyMembers.JoinedAt           AS @JoinedAt,
            Users.Username      AS @Username,
            Users.Discriminator AS @Discriminator,
            Users.Flags         AS @UserFlags,
            .combine_profile_bits(BaseProfile.Bits, PartyProfile.Bits, PartyProfile.AvatarId) AS @ProfileBits,
            COALESCE(PartyProfile.AvatarId, BaseProfile.AvatarId) AS @AvatarId,
            COALESCE(PartyProfile.Nickname, BaseProfile.Nickname) AS @Nickname,
            SelectedMessages.Content        AS @Content,
            AggMentions.Kinds       AS @MentionKinds,
            AggMentions.Ids         AS @MentionIds,
            (
                SELECT ARRAY_AGG(RoleMembers.RoleId)
                FROM RoleMembers INNER JOIN Roles ON Roles.Id = RoleMembers.RoleId
                WHERE RoleMembers.UserId = SelectedMessages.UserId
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
                    "m", match search {
                        SearchRequest::Single { .. } => { FALSE },
                        _ => { ReactionUsers.UserId IS NOT NULL }
                    },
                    "c", AggReactions.Count
                )) FROM AggReactions

                // where a user_id is available, check for own reaction in ReactionUsers
                if let Some(user_id) = search.user_id() {
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

        FROM SelectedMessages
            INNER JOIN Users ON Users.Id = SelectedMessages.UserId
            LEFT JOIN Profiles AS BaseProfile
                ON BaseProfile.UserId = SelectedMessages.UserId AND BaseProfile.PartyId IS NULL
            LEFT JOIN Profiles AS PartyProfile
                ON PartyProfile.UserId = SelectedMessages.UserId AND PartyProfile.PartyId = SelectedMessages.PartyId
            // PartyId can be null for non-party room messages
            LEFT JOIN PartyMembers ON PartyMembers.UserId = SelectedMessages.UserId
                AND PartyMembers.PartyId IS NOT DISTINCT FROM SelectedMessages.PartyId
            LEFT JOIN AggMentions ON AggMentions.MsgId = SelectedMessages.Id

            if let Some(user_id) = search.user_id() {
                LEFT JOIN AggRelationships
                    ON AggRelationships.UserId = SelectedMessages.UserId
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
                WHERE Attachments.MessageId = SelectedMessages.Id
            ) AS TempAttachments ON TRUE

        LIMIT {limit.min(100)}
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
    query: Option<SmolStr>,
    starred: bool,
    order: Sort,
    prefers_post_sort: bool,
}

fn process_terms(
    state: &ServerState,
    auth: &Authorization,
    terms: &mut SearchTerms,
    scope: &mut SearchScope,
) -> Result<ProcessedSearch, Error> {
    let mut data = ProcessedSearch::default();

    let mut err = None;

    #[allow(clippy::match_like_matches_macro)]
    terms.retain(|term| match term.kind {
        SearchTermKind::Query(ref tsquery) => {
            data.query = Some(tsquery.clone());
            true
        }
        SearchTermKind::Prefix(_) => true,
        SearchTermKind::Regex(ref re) => {
            if !auth.flags.intersects(UserFlags::PREMIUM) {
                err = Some(Error::Unauthorized);
            } else if re.len() > state.config().message.max_regex_search_len as usize {
                err = Some(Error::SearchError(SearchError::InvalidRegex));
            }

            true
        }
        SearchTermKind::Before(_) => true,
        SearchTermKind::After(_) => true,
        SearchTermKind::Sort(order) => {
            data.order = order;
            false
        }
        SearchTermKind::User(_) => true,
        SearchTermKind::Room(id) => {
            // since we limit to rooms anyway
            *scope = SearchScope::Room(id, true);
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
                Has::Code | Has::Text => return true,
            }

            false
        }
        _ => false,
    });

    if matches!(data.order, Sort::Relevant if data.query.is_none()) {
        data.order = Sort::Descending;
    }

    if data.query.is_some() {
        data.prefers_post_sort = true;
    } else {
        for term in terms.iter() {
            if matches!(term.kind, SearchTermKind::Prefix(_) | SearchTermKind::Regex(_)) {
                data.prefers_post_sort = true;
                break;
            }
        }
    }

    match err {
        Some(e) => Err(e),
        None => Ok(data),
    }
}
