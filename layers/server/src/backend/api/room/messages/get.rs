use arrayvec::ArrayVec;
use db::{
    pg::Statement,
    pool::{Client, Object, Transaction},
};
use futures::{FutureExt, Stream, StreamExt};

use schema::{flags::AttachmentFlags, Snowflake, SnowflakeExt};
use sdk::models::*;
use thorn::pg::{Json, ToSql};

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

use sdk::api::commands::room::GetMessagesQuery;

pub async fn get_one(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> Result<Message, Error> {
    let stream = get_many(
        state,
        auth,
        room_id,
        GetMessagesQuery {
            query: Some(Cursor::Exact(msg_id)),
            limit: Some(1),
            thread: None,
            pinned: Vec::new(),
            starred: false,
        },
    )
    .await?;

    futures::pin_mut!(stream);

    match stream.next().await {
        Some(res) => res,
        None => Err(Error::NotFound),
    }
}

pub async fn get_many(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    form: GetMessagesQuery,
) -> Result<impl Stream<Item = Result<Message, Error>>, Error> {
    let had_perms = match state.perm_cache.get(auth.user_id, room_id).await {
        Some(perm) => {
            if !perm.contains(RoomPermissions::READ_MESSAGE_HISTORY) {
                return Err(Error::NotFound);
            }

            true
        }
        None => false,
    };

    let db = state.db.read.get().await?;

    let msg_id = match form.query {
        Some(Cursor::After(id) | Cursor::Before(id) | Cursor::Exact(id)) => id,
        None => Snowflake::max_value(),
    };

    let limit = match form.limit {
        Some(limit) => 100.min(limit as i16),
        None => 100,
    };

    #[rustfmt::skip]
    let query = match (had_perms, form.query) {
        (true,  None | Some(Cursor::Before(_))) => db.prepare_cached_typed(p::wuser_before_no_perm).boxed(),
        (true,         Some(Cursor::After(_)))  => db.prepare_cached_typed(p::wuser_after_no_perm).boxed(),
        (true,         Some(Cursor::Exact(_)))  => db.prepare_cached_typed(p::wuser_exact_no_perm).boxed(),
        (false, None | Some(Cursor::Before(_))) => db.prepare_cached_typed(p::wuser_before_perm).boxed(),
        (false,        Some(Cursor::After(_)))  => db.prepare_cached_typed(p::wuser_after_perm).boxed(),
        (false,        Some(Cursor::Exact(_)))  => db.prepare_cached_typed(p::wuser_exact_perm).boxed(),
    };

    let query = query.await?;

    use q::{Parameters, Params};

    let params = Params {
        room_id: Some(room_id),
        msg_id,
        limit,
        user_id: Some(auth.user_id),
        thread_id: form.thread,
        pinned: form.pinned,
        starred: form.starred,
    };

    Ok(parse_stream(state, db.query_stream(&query, &params.as_params()).await?))
}

pub async fn get_one_transactional(
    state: ServerState,
    msg_id: Snowflake,
    room_id: Snowflake,
    t: &db::pool::Transaction<'_>,
) -> Result<Message, Error> {
    use q::{Parameters, Params};

    let params = Params {
        room_id: Some(room_id),
        msg_id,
        limit: 1,
        user_id: None,
        thread_id: None,
        pinned: Vec::new(),
        starred: false,
    };

    parse_first(
        state,
        t.query_stream_cached_typed(p::exact_no_perm, &params.as_params())
            .await?,
    )
    .await
}

pub async fn get_one_from_client(
    state: ServerState,
    msg_id: Snowflake,
    db: &db::pool::Client,
) -> Result<Message, Error> {
    use q::{Parameters, Params};

    let params = Params {
        room_id: None,
        msg_id,
        limit: 1,
        user_id: None,
        thread_id: None,
        pinned: Vec::new(),
        starred: false,
    };

    parse_first(
        state,
        db.query_stream_cached_typed(p::exact_no_perm_no_room, &params.as_params())
            .await?,
    )
    .await
}

#[rustfmt::skip]
mod p {
    use super::*;
    use thorn::AnyQuery;

    use Cursor::*;
    const NULL: Snowflake = Snowflake::null();

    pub fn exact_no_perm_no_room() -> impl AnyQuery { q::query(Exact(NULL), false, false, false) }
    pub fn exact_no_perm()  -> impl AnyQuery { q::query(Exact(NULL),  false, true, false) }

    pub fn wuser_before_no_perm() -> impl AnyQuery { q::query(Before(NULL), false, true, true) }
    pub fn wuser_after_no_perm()  -> impl AnyQuery { q::query(After(NULL),  false, true, true) }
    pub fn wuser_exact_no_perm()  -> impl AnyQuery { q::query(Exact(NULL),  false, true, true) }
    pub fn wuser_before_perm()    -> impl AnyQuery { q::query(Before(NULL), true, true, true) }
    pub fn wuser_after_perm()     -> impl AnyQuery { q::query(After(NULL),  true, true, true) }
    pub fn wuser_exact_perm()     -> impl AnyQuery { q::query(Exact(NULL),  true, true, true) }
}

mod q {
    use super::{Cursor, MessageFlags, Permission};
    use db::Row;
    use sdk::models::UserRelationship;

    pub use schema::*;
    pub use thorn::*;

    thorn::tables! {
        pub struct SelectedMessages {
            Id: Messages::Id,
            Unavailable: Type::BOOL,
            Starred: Type::BOOL,
        }

        pub struct SelectStarred {
            Starred: Type::BOOL,
        }

        struct AggPerm {
            Perms: AggRoomPerms::Perms,
        }

        pub struct TempReactions {
            Reactions: Type::JSONB,
        }

        pub struct TempParty {
            Id: Party::Id,
            RoomId: Rooms::Id,
        }
    }

    thorn::decl_alias! {
        pub BaseProfile = Profiles,
        pub PartyProfile = Profiles
    }

    pub mod columns {
        use super::*;

        thorn::indexed_columns! {
            pub enum MessageColumns {
                Messages::Id,
                Messages::UserId,
                Messages::RoomId,
                Messages::Kind,
                Messages::ThreadId,
                Messages::EditedAt,
                Messages::Flags,
                Messages::Embeds,
            }

            pub enum SelectedColumns continue MessageColumns {
                SelectedMessages::Unavailable,
                SelectedMessages::Starred,
            }

            pub enum PartyColumns continue SelectedColumns {
                TempParty::Id,
            }

            pub enum MemberColumns continue PartyColumns {
                AggMembers::JoinedAt,
            }

            pub enum UserColumns continue MemberColumns {
                Users::Username,
                Users::Discriminator,
                Users::Flags,
            }

            pub enum ProfileColumns continue UserColumns {
                Profiles::Bits,
                Profiles::AvatarId,
                Profiles::Nickname,
            }

            pub enum MentionColumns continue ProfileColumns {
                AggMentions::Kinds,
                AggMentions::Ids,
            }

            pub enum DynamicMsgColumns continue MentionColumns {
                Messages::PinTags,
                Messages::Content,
            }

            pub enum RoleColumns continue DynamicMsgColumns {
                AggMembers::RoleIds,
            }

            pub enum AttachmentColumns continue RoleColumns {
                AggAttachments::Meta,
                AggAttachments::Preview,
            }

            pub enum ReactionColumns continue AttachmentColumns {
                TempReactions::Reactions,
            }
        }
    }

    use columns::*;

    thorn::params! {
        pub struct Params {
            pub msg_id: Snowflake = Messages::Id,
            pub limit: i16 = Type::INT2,
            pub room_id: Option<Snowflake> = Rooms::Id,
            pub user_id: Option<Snowflake> = Users::Id,
            pub thread_id: Option<Snowflake> = Threads::Id,
            pub pinned: Vec<Snowflake> = SNOWFLAKE_ARRAY,
            pub starred: bool = Type::BOOL,
        }
    }

    pub fn query(
        mode: Cursor,
        check_perms: bool,
        with_room: bool, // messages selected from a known room
        with_user: bool, // messages selected by a known user
    ) -> impl thorn::AnyQuery {
        let mut selected = Query::select()
            .expr(Messages::Id.alias_to(SelectedMessages::Id))
            .expr(SelectStarred::Starred.alias_to(SelectedMessages::Starred))
            // test if message is deleted
            .and_where(Messages::Flags.has_no_bits(MessageFlags::DELETED.bits().lit()))
            .and_where(
                Params::thread_id()
                    .is_null()
                    .or(Messages::ThreadId.equals(Params::thread_id())),
            )
            .and_where(
                Builtin::cardinality(Params::pinned())
                    .equals(0.lit())
                    .or(Messages::PinTags.overlaps(Params::pinned())),
            )
            .and_where(Params::starred().is_false().or(SelectStarred::Starred))
            .limit(Params::limit());

        selected = match with_user {
            false => selected
                .from_table::<Messages>()
                .expr(false.lit().alias_to(SelectedMessages::Unavailable)),
            true => selected
                .from(
                    Messages::left_join_table::<AggRelationships>().on(AggRelationships::UserId
                        .equals(Messages::UserId)
                        .and(AggRelationships::FriendId.equals(Params::user_id()))),
                )
                // if user of message has reported reader as dangerous
                .expr(
                    AggRelationships::RelA
                        .equals((UserRelationship::BlockedDangerous as i8).lit())
                        .is_true()
                        .alias_to(SelectedMessages::Unavailable),
                ),
        };

        // Need to shove this logic into a lateral subquery so it can be used in WHERE
        selected = selected.from(Lateral(SelectStarred::as_query(
            Query::select().expr(
                Builtin::coalesce((Params::user_id().equals(Builtin::any(Messages::StarredBy)), false.lit()))
                    .alias_to(SelectStarred::Starred),
            ),
        )));

        if with_room {
            selected = selected.and_where(Messages::RoomId.equals(Params::room_id()));
        } else {
            // if there is no room to select from, double-check that we're picking out
            // a single message
            debug_assert!(matches!(mode, Cursor::Exact(_)));
        }

        selected = match mode {
            Cursor::After(_) => selected
                .and_where(Messages::Id.greater_than(Params::msg_id()))
                .order_by(Messages::Id.ascending()),

            Cursor::Before(_) => selected
                .and_where(Messages::Id.less_than(Params::msg_id()))
                .order_by(Messages::Id.descending()),

            Cursor::Exact(_) => selected.and_where(Messages::Id.equals(Params::msg_id())),
        };

        let party = match with_room {
            true => Query::select()
                .expr(Rooms::PartyId.alias_to(TempParty::Id))
                .expr(Rooms::Id.alias_to(TempParty::RoomId))
                .from_table::<Rooms>()
                .and_where(Rooms::Id.equals(Params::room_id())),

            false => Query::select()
                .expr(Rooms::PartyId.alias_to(TempParty::Id))
                .expr(Rooms::Id.alias_to(TempParty::RoomId))
                .from_table::<Rooms>()
                .and_where(Params::room_id().is_null()),
        };

        #[rustfmt::skip]
        let mut query = Query::select()
            .with(SelectedMessages::as_query(selected.materialized()).exclude())
            .with(TempParty::as_query(party).exclude())
            .cols(MessageColumns::default())
            .cols(SelectedColumns::default())
            .cols(PartyColumns::default())
            .cols(MemberColumns::default())
            .cols(UserColumns::default())
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
            .cols(MentionColumns::default())
            .cols(DynamicMsgColumns::default())
            .cols(RoleColumns::default())
            .cols(AttachmentColumns::default())
            // ReactionColumns
            .expr(
                Query::select()
                    .expr(Call::custom("jsonb_agg").arg(
                        Call::custom("jsonb_build_object").args((
                            "emote_id".lit(), AggReactions::EmoteId,
                            "emoji_id".lit(), AggReactions::EmojiId,
                            "me".lit(), Params::user_id().equals(Builtin::any(AggReactions::UserIds)),
                            "count".lit(), Builtin::coalesce((
                                Builtin::array_length((AggReactions::UserIds, 1.lit())), 0.lit()
                            ))
                        ))),
                    )
                    .from_table::<AggReactions>()
                    .and_where(AggReactions::MsgId.equals(Messages::Id))
                    .as_value(),
            )
            .from(
                Messages::inner_join_table::<SelectedMessages>()
                    .on(Messages::Id.equals(SelectedMessages::Id))
                    .inner_join_table::<TempParty>()
                    .on(TempParty::RoomId.equals(Messages::RoomId))
                    .inner_join_table::<Users>()
                    .on(Users::Id.equals(Messages::UserId))
                    .left_join_table::<BaseProfile>()
                    .on(BaseProfile::col(Profiles::UserId)
                        .equals(Messages::UserId)
                        .and(BaseProfile::col(Profiles::PartyId).is_null()))
                    .left_join_table::<PartyProfile>()
                    .on(PartyProfile::col(Profiles::UserId)
                        .equals(Messages::UserId)
                        .and(PartyProfile::col(Profiles::PartyId).equals(TempParty::Id)))
                    .left_join_table::<AggMembers>()
                    .on(AggMembers::UserId.equals(Messages::UserId).and(
                        AggMembers::PartyId
                            .equals(TempParty::Id)
                            .or(AggMembers::PartyId.is_null().and(TempParty::Id.is_null())),
                    ))
                    .left_join_table::<AggAttachments>()
                    .on(AggAttachments::MsgId.equals(Messages::Id))
                    .left_join_table::<AggMentions>()
                    .on(AggMentions::MsgId.equals(Messages::Id)),
            );

        if check_perms {
            const READ_MESSAGES: i64 = Permission::PACKED_READ_MESSAGE_HISTORY as i64;

            query = query
                .with(AggPerm::as_query(
                    Query::select()
                        .expr(AggRoomPerms::Perms.alias_to(AggPerm::Perms))
                        .from_table::<AggRoomPerms>()
                        .and_where(AggRoomPerms::UserId.equals(Params::user_id()))
                        .and_where(
                            AggRoomPerms::RoomId
                                .equals(Params::room_id())
                                .or(Params::room_id().is_null()),
                        ),
                ))
                .and_where(AggPerm::Perms.has_all_bits(READ_MESSAGES.lit()))
        }

        query
    }
}

pub async fn parse_first<S>(state: ServerState, stream: S) -> Result<Message, Error>
where
    S: Stream<Item = Result<db::Row, db::pool::Error>>,
{
    let msg_stream = parse_stream(state, stream);

    futures::pin_mut!(msg_stream);

    match msg_stream.next().await {
        Some(res) => res,
        None => Err(Error::NotFound),
    }
}

pub fn parse_stream<S>(state: ServerState, stream: S) -> impl Stream<Item = Result<Message, Error>>
where
    S: Stream<Item = Result<db::Row, db::pool::Error>>,
{
    // for many messages from the same user in a row, avoid duplicating work of encoding user things at the cost of cloning it
    let mut last_user: Option<User> = None;

    use q::columns::*;

    stream.map(move |row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => {
            let party_id: Option<Snowflake> = row.try_get(PartyColumns::id())?;
            let msg_id = row.try_get(MessageColumns::id())?;
            let unavailable = row.try_get(SelectedColumns::unavailable())?;

            // many fields here are empty, easy to construct, and are filled in below
            let mut msg = Message {
                id: msg_id,
                party_id,
                created_at: msg_id.timestamp(),
                room_id: row.try_get(MessageColumns::room_id())?,
                flags: MessageFlags::empty(),
                kind: MessageKind::Normal,
                edited_at: None,
                content: None,
                author: make_system_user(),
                member: None,
                thread_id: row.try_get(MessageColumns::thread_id())?,
                user_mentions: Vec::new(),
                role_mentions: Vec::new(),
                room_mentions: Vec::new(),
                attachments: Vec::new(),
                reactions: Vec::new(),
                embeds: Vec::new(),
                pins: Vec::new(),
                starred: false,
            };

            // before we continue, if the message was marked unavailable, then we can skip everything else
            if unavailable {
                msg.kind = MessageKind::Unavailable;

                return Ok(msg);
            }

            msg.author = {
                let id = row.try_get(MessageColumns::user_id())?;

                match last_user {
                    Some(ref last_user) if last_user.id == id => last_user.clone(),
                    _ => {
                        let user = User {
                            id,
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
                                    avatar: encrypt_snowflake_opt(
                                        &state,
                                        row.try_get(ProfileColumns::avatar_id())?,
                                    )
                                    .into(),
                                    banner: Nullable::Undefined,
                                    status: Nullable::Undefined,
                                    bio: Nullable::Undefined,
                                }),
                            },
                        };

                        last_user = Some(user.clone());

                        user
                    }
                }
            };

            msg.flags = MessageFlags::from_bits_truncate_public(row.try_get(MessageColumns::flags())?);
            msg.kind = MessageKind::try_from(row.try_get::<_, i16>(MessageColumns::kind())?).unwrap_or_default();

            msg.member = match party_id {
                None => None,
                Some(_) => Some(PartialPartyMember {
                    roles: row.try_get(RoleColumns::role_ids())?,
                    joined_at: row.try_get(MemberColumns::joined_at())?,
                    flags: None,
                }),
            };

            msg.content = row.try_get(DynamicMsgColumns::content())?;
            msg.edited_at = row.try_get(MessageColumns::edited_at())?;

            let mut user_mentions = &mut msg.user_mentions;
            let mut role_mentions = &mut msg.role_mentions;
            let mut room_mentions = &mut msg.room_mentions;

            let mention_kinds: Option<Vec<i32>> = row.try_get(MentionColumns::kinds())?;

            match mention_kinds {
                Some(mention_kinds) if !mention_kinds.is_empty() => {
                    let mention_ids: Vec<Snowflake> = row.try_get(MentionColumns::ids())?;

                    if mention_kinds.len() != mention_ids.len() {
                        return Err(Error::InternalErrorStatic(
                            "Mismatch in number of mention ids and mention kinds",
                        ));
                    }

                    for (kind, id) in mention_kinds.into_iter().zip(mention_ids) {
                        let mentions = match kind {
                            1 => &mut user_mentions,
                            2 => &mut role_mentions,
                            3 => &mut room_mentions,
                            _ => unreachable!(),
                        };

                        mentions.push(id);
                    }
                }
                _ => {}
            }

            msg.attachments = {
                let mut attachments = Vec::new();

                let meta: Option<Json<Vec<schema::AggAttachmentsMeta>>> = row.try_get(AttachmentColumns::meta())?;

                if let Some(Json(meta)) = meta {
                    let previews: Vec<Option<&[u8]>> = row.try_get(AttachmentColumns::preview())?;

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
                                preview: preview.and_then(|p| p.to_z85().ok()),
                            },
                        })
                    }
                }

                attachments
            };

            msg.pins = row
                .try_get::<_, Option<Vec<Snowflake>>>(DynamicMsgColumns::pin_tags())?
                .unwrap_or_default();

            msg.starred = row.try_get(SelectedColumns::starred())?;

            msg.reactions = match row.try_get(ReactionColumns::reactions())? {
                Some(Json::<Vec<RawReaction>>(raw)) if !raw.is_empty() => {
                    let mut reactions = Vec::with_capacity(raw.len());

                    for r in raw {
                        if r.count == 0 {
                            continue;
                        }

                        reactions.push(Reaction::Shorthand(ReactionShorthand {
                            me: r.me,
                            count: r.count,
                            emote: match (r.emote_id, r.emoji_id) {
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

                    reactions
                }
                _ => Vec::new(),
            };

            let mention_kinds: Option<Vec<i32>> = row.try_get(MentionColumns::kinds())?;
            if let Some(mention_kinds) = mention_kinds {
                // lazily parse ids
                let mention_ids: Vec<Snowflake> = row.try_get(MentionColumns::ids())?;

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

            if let Some(Json(embeds)) = row.try_get(MessageColumns::embeds())? {
                msg.embeds = embeds;
            }

            Ok(msg)
        }
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
    pub emote_id: Option<Snowflake>,
    pub emoji_id: Option<i32>,
    pub me: bool,
    pub count: i64,
}
