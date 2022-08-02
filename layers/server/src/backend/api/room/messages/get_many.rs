use arrayvec::ArrayVec;
use futures::{FutureExt, Stream, StreamExt};

use schema::{flags::AttachmentFlags, Snowflake, SnowflakeExt};
use sdk::models::*;
use thorn::pg::{Json, ToSql};

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

use sdk::api::commands::room::{GetMessagesQuery, MessageSearch};

pub async fn get_many(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    form: GetMessagesQuery,
) -> Result<impl Stream<Item = Result<Message, Error>>, Error> {
    let had_perms = if let Some(perm) = state.perm_cache.get(auth.user_id, room_id).await {
        if !perm.contains(RoomPermissions::READ_MESSAGE_HISTORY) {
            return Err(Error::NotFound);
        }

        true
    } else {
        false
    };

    let db = state.db.read.get().await?;

    let msg_id = match form.query {
        Some(MessageSearch::After(id)) => id,
        Some(MessageSearch::Before(id)) => id,
        None => Snowflake::max_value(),
    };

    let limit = match form.limit {
        Some(limit) => 100.min(limit as i16),
        None => 50,
    };

    #[rustfmt::skip]
    let query = {
        use MessageSearch::*;
        const NULL: Snowflake = Snowflake::null();

        match (had_perms, form.query) {
            (true,  None | Some(MessageSearch::Before(_))) => db.prepare_cached_typed(|| query(Before(NULL), false)).boxed(),
            (true,         Some(MessageSearch::After(_)))  => db.prepare_cached_typed(|| query(After(NULL), false)).boxed(),
            (false, None | Some(MessageSearch::Before(_))) => db.prepare_cached_typed(|| query(Before(NULL), true)).boxed(),
            (false,        Some(MessageSearch::After(_)))  => db.prepare_cached_typed(|| query(After(NULL), true)).boxed(),
        }
    };

    use q::{Parameters, Params};

    let stream = db
        .query_stream(
            &query.await?,
            &Params {
                room_id,
                msg_id,
                limit,
                user_id: auth.user_id,
                thread_id: form.thread,
            }
            .as_params(),
        )
        .await?;

    // for many messages from the same user in a row, avoid duplicating work of encoding user things at the cost of cloning it
    let mut last_user: Option<User> = None;

    use q::{Columns, ReactionColumns};

    Ok(stream.map(move |row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => {
            let party_id: Option<Snowflake> = row.try_get(Columns::party_id())?;
            let msg_id = row.try_get(Columns::msg_id())?;

            let mut msg = Message {
                id: msg_id,
                party_id,
                created_at: msg_id.timestamp(),
                room_id,
                flags: MessageFlags::from_bits_truncate_public(row.try_get(Columns::message_flags())?),
                kind: MessageKind::try_from(row.try_get::<_, i16>(Columns::kind())?).unwrap_or_default(),
                edited_at: row.try_get::<_, Option<_>>(Columns::edited_at())?,
                content: row.try_get(Columns::content())?,
                author: {
                    let id = row.try_get(Columns::user_id())?;

                    match last_user {
                        Some(ref last_user) if last_user.id == id => last_user.clone(),
                        _ => {
                            let user = User {
                                id,
                                username: row.try_get(Columns::username())?,
                                discriminator: row.try_get(Columns::discriminator())?,
                                flags: UserFlags::from_bits_truncate_public(
                                    row.try_get(Columns::user_flags())?,
                                ),
                                email: None,
                                preferences: None,
                                profile: match row.try_get(Columns::profile_bits())? {
                                    None => Nullable::Null,
                                    Some(bits) => Nullable::Some(UserProfile {
                                        bits,
                                        avatar: encrypt_snowflake_opt(
                                            &state,
                                            row.try_get(Columns::avatar_id())?,
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
                },
                member: match party_id {
                    None => None,
                    Some(_) => Some(PartyMember {
                        user: None,
                        nick: row.try_get(Columns::Nickname as usize)?,
                        roles: row.try_get(Columns::RoleIds as usize)?,
                        presence: None,
                        flags: None,
                    }),
                },
                thread_id: row.try_get(Columns::ThreadId as usize)?,
                user_mentions: Vec::new(),
                role_mentions: Vec::new(),
                room_mentions: Vec::new(),
                attachments: {
                    let mut attachments = Vec::new();

                    let meta: Option<Json<Vec<schema::AggAttachmentsMeta>>> =
                        row.try_get(Columns::AttachmentMeta as usize)?;

                    if let Some(Json(meta)) = meta {
                        let previews: Vec<Option<&[u8]>> =
                            row.try_get(Columns::AttachmentPreview as usize)?;

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
                },
                reactions: match row.try_get(ReactionColumns::Reactions as usize)? {
                    Some(Json(reactions)) => reactions,
                    None => Vec::new(),
                },
                embeds: Vec::new(),
                pins: row
                    .try_get::<_, Option<Vec<Snowflake>>>(Columns::pin_tags())?
                    .unwrap_or_default(),
            };

            let mention_kinds: Option<Vec<i32>> = row.try_get(Columns::MentionKinds as usize)?;
            if let Some(mention_kinds) = mention_kinds {
                // lazily parse ids
                let mention_ids: Vec<Snowflake> = row.try_get(Columns::MentionIds as usize)?;

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

            Ok(msg)
        }
    }))
}

mod q {
    pub use schema::*;
    pub use thorn::*;

    thorn::tables! {
        pub struct TempReactions {
            MsgId: Reactions::MsgId,
            Reactions: Type::JSONB,
        }
    }

    thorn::indexed_columns! {
        pub enum Columns {
            AggMessages::MsgId,
            AggMessages::UserId,
            AggMessages::PartyId,
            AggMessages::Kind,
            AggMessages::Nickname,
            AggMessages::Username,
            AggMessages::Discriminator,
            AggMessages::UserFlags,
            AggMessages::EditedAt,
            AggMessages::MessageFlags,
            AggMessages::AvatarId,
            AggMessages::ProfileBits,
            AggMessages::ThreadId,
            AggMessages::Content,
            AggMessages::RoleIds,
            AggMessages::PinTags,
            AggMessages::MentionIds,
            AggMessages::MentionKinds,
            AggMessages::AttachmentMeta,
            AggMessages::AttachmentPreview,
        }

        pub enum ReactionColumns continue Columns {
            TempReactions::Reactions,
        }
    }

    thorn::params! {
        pub struct Params {
            pub room_id: Snowflake = Rooms::Id,
            pub msg_id: Snowflake = Messages::Id,
            pub limit: i16 = Type::INT2,
            pub user_id: Snowflake = Users::Id,
            pub thread_id: Option<Snowflake> = Threads::Id,
        }
    }
}

fn query(mode: MessageSearch, check_perms: bool) -> impl thorn::AnyQuery {
    use q::*;

    tables! {
        struct AggNumberedMsg {
            MsgId: AggMessages::MsgId,
            RowNumber: Type::INT8,
        }

        pub struct SelectedMessages {
            Id: Messages::Id,
        }

        struct AggPerm {
            Perms: AggRoomPerms::Perms,
        }
    }

    let mut selected = Query::select()
        .expr(Messages::Id.alias_to(SelectedMessages::Id))
        .from_table::<Messages>()
        .and_where(Messages::RoomId.equals(Params::room_id()))
        .and_where(
            // test if message is deleted
            Messages::Flags
                .bit_and(MessageFlags::DELETED.bits().lit())
                .equals(0i16.lit()),
        )
        .and_where(
            Params::thread_id()
                .is_null()
                .or(Messages::ThreadId.equals(Params::thread_id())),
        )
        .limit(Params::limit());

    selected = match mode {
        MessageSearch::After(_) => selected
            .and_where(Messages::Id.greater_than(Params::msg_id()))
            .order_by(Messages::Id.ascending()),

        MessageSearch::Before(_) => selected
            .and_where(Messages::Id.less_than(Params::msg_id()))
            .order_by(Messages::Id.descending()),
    };

    let mut query = Query::select()
        .with(SelectedMessages::as_query(selected).exclude())
        .cols(Columns::default())
        .cols(ReactionColumns::default())
        .from(
            AggMessages::inner_join_table::<SelectedMessages>()
                .on(AggMessages::MsgId.equals(SelectedMessages::Id))
                .left_join(Lateral(TempReactions::as_query(
                    // TODO: Move this into view
                    Query::select()
                        .expr(Reactions::MsgId.alias_to(TempReactions::MsgId))
                        .expr(
                            Call::custom("jsonb_agg")
                                .arg(
                                    Call::custom("jsonb_build_object")
                                        .arg("emote".lit())
                                        .arg(Reactions::EmoteId)
                                        .arg("own".lit())
                                        .arg(Params::user_id().equals(Builtin::any(Reactions::UserIds)))
                                        .arg("count".lit())
                                        .arg(
                                            Call::custom("array_length")
                                                .arg(Reactions::UserIds)
                                                .arg(1i16.lit()),
                                        ),
                                )
                                .alias_to(TempReactions::Reactions),
                        )
                        .from_table::<Reactions>()
                        .group_by(Reactions::MsgId),
                )))
                .on(TempReactions::MsgId.equals(SelectedMessages::Id)),
        );

    if check_perms {
        const READ_MESSAGES: i64 = Permission::PACKED_READ_MESSAGE_HISTORY as i64;

        query = query
            .with(AggPerm::as_query(
                Query::select()
                    .expr(AggRoomPerms::Perms.alias_to(AggPerm::Perms))
                    .from_table::<AggRoomPerms>()
                    .and_where(AggRoomPerms::UserId.equals(Params::user_id()))
                    .and_where(AggRoomPerms::RoomId.equals(Params::room_id())),
            ))
            .and_where(
                AggPerm::Perms
                    .bit_and(READ_MESSAGES.lit())
                    .equals(READ_MESSAGES.lit()),
            )
    }

    query
}
