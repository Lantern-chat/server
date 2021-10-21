use schema::{Snowflake, SnowflakeExt};

use futures::{Stream, StreamExt};
use models::*;
use thorn::pg::{Json, ToSql};

use crate::{
    ctrl::{
        perm::get_cached_room_permissions_with_conn, util::encrypted_asset::encrypt_snowflake_opt, Error,
    },
    web::auth::Authorization,
    ServerState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageSearch {
    After(Snowflake),
    Before(Snowflake),
    Around(Snowflake),
}

#[derive(Deserialize)]
pub struct GetManyMessagesForm {
    #[serde(flatten)]
    query: Option<MessageSearch>,

    #[serde(default = "default_limit")]
    limit: u8,
}

#[rustfmt::skip]
const fn default_limit() -> u8 { 50 }

impl Default for GetManyMessagesForm {
    fn default() -> Self {
        GetManyMessagesForm {
            query: None,
            limit: default_limit(),
        }
    }
}

pub async fn get_many(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    form: GetManyMessagesForm,
) -> Result<impl Stream<Item = Result<Message, Error>>, Error> {
    let had_perms = if let Some(perm) = state.perm_cache.get(auth.user_id, room_id).await {
        if !perm.perm.room.contains(RoomPermissions::READ_MESSAGES) {
            return Err(Error::NotFound);
        }

        true
    } else {
        false
    };

    let db = state.db.read.get().await?;

    let msg_id = match form.query {
        Some(MessageSearch::After(id)) => id,
        Some(MessageSearch::Around(id)) => id,
        Some(MessageSearch::Before(id)) => id,
        None => Snowflake::max_value(),
    };

    let limit = 100.min(form.limit as i16);

    let limit = match form.query {
        Some(MessageSearch::Around(_)) => (limit + limit % 2) / 2,
        _ => limit,
    };

    use arrayvec::ArrayVec;

    let mut params: ArrayVec<&(dyn ToSql + Sync), 4> =
        ArrayVec::from([&room_id as _, &msg_id as _, &limit as _, &auth.user_id as _]);

    let query = if had_perms {
        params.pop(); // remove auth.user_id for these queries

        use MessageSearch::*;
        const NULL: Snowflake = Snowflake::null();

        match form.query {
            None | Some(MessageSearch::Before(_)) => {
                db.prepare_cached_typed(|| query(Before(NULL), false)).await
            }
            Some(MessageSearch::After(_)) => db.prepare_cached_typed(|| query(After(NULL), false)).await,
            Some(MessageSearch::Around(_)) => db.prepare_cached_typed(|| query(Around(NULL), false)).await,
        }
    } else {
        use MessageSearch::*;
        const NULL: Snowflake = Snowflake::null();

        match form.query {
            None | Some(MessageSearch::Before(_)) => {
                db.prepare_cached_typed(|| query(Before(NULL), true)).await
            }
            Some(MessageSearch::After(_)) => db.prepare_cached_typed(|| query(After(NULL), true)).await,
            Some(MessageSearch::Around(_)) => db.prepare_cached_typed(|| query(Around(NULL), true)).await,
        }
    };

    let stream = db.query_stream(&query?, &params).await?;

    // for many messages from the same user in a row, avoid duplicating work of encoding user things at the cost of cloning it
    let mut last_user: Option<User> = None;

    Ok(stream.map(move |row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => {
            let party_id: Option<Snowflake> = row.try_get(2)?;
            let msg_id = row.try_get(0)?;

            let mut msg = Message {
                id: msg_id,
                party_id,
                created_at: msg_id.timestamp(),
                room_id,
                flags: MessageFlags::from_bits_truncate(row.try_get(10)?),
                edited_at: row.try_get::<_, Option<_>>(9)?,
                content: row.try_get(11)?,
                author: {
                    let id = row.try_get(1)?;

                    match last_user {
                        Some(ref last_user) if last_user.id == id => last_user.clone(),
                        _ => {
                            let user = User {
                                id,
                                username: row.try_get(4)?,
                                discriminator: row.try_get(5)?,
                                flags: UserFlags::from_bits_truncate(row.try_get(6)?).publicize(),
                                status: None,
                                bio: None,
                                email: None,
                                preferences: None,
                                avatar: encrypt_snowflake_opt(&state, row.try_get(13)?),
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
                        nick: row.try_get(3)?,
                        roles: row.try_get(12)?,
                        presence: None,
                    }),
                },
                thread_id: None,
                user_mentions: Vec::new(),
                role_mentions: Vec::new(),
                room_mentions: Vec::new(),
                attachments: {
                    let mut attachments = Vec::new();

                    let meta: Option<Json<Vec<schema::AggAttachmentsMeta>>> = row.try_get(14)?;

                    if let Some(Json(meta)) = meta {
                        let previews: Vec<Option<&[u8]>> = row.try_get(15)?;

                        if meta.len() != previews.len() {
                            return Err(Error::InternalErrorStatic("Meta != Previews length"));
                        }

                        attachments.reserve(meta.len());

                        for (meta, preview) in meta.into_iter().zip(previews) {
                            use blurhash::base85::ToZ85;

                            attachments.push(Attachment {
                                id: meta.id,
                                filename: meta.name,
                                size: meta.size as usize,
                                mime: meta.mime,
                                embed: EmbedMediaAttributes {
                                    preview: preview.and_then(|p| p.to_z85().ok()),
                                    ..EmbedMediaAttributes::default()
                                },
                            })
                        }
                    }

                    attachments
                },
                embeds: Vec::new(),
                reactions: Vec::new(),
            };

            let mention_kinds: Option<Vec<i32>> = row.try_get(8)?;
            if let Some(mention_kinds) = mention_kinds {
                // lazily parse ids
                let mention_ids: Vec<Snowflake> = row.try_get(7)?;

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

fn query(mode: MessageSearch, check_perms: bool) -> impl thorn::AnyQuery {
    use schema::*;
    use thorn::*;

    tables! {
        struct AggNumberedMsg {
            MsgId: AggMessages::MsgId,
            RowNumber: Type::INT8,
        }
    }

    let room_id_var = Var::at(Rooms::Id, 1);
    let msg_id_var = Var::at(Messages::Id, 2);
    let limit_var = Var::at(Type::INT2, 3);
    let user_id_var = Var::at(Users::Id, 4);

    let mut query = Query::select()
        .cols(&[
            /* 0*/ AggMessages::MsgId,
            /* 1*/ AggMessages::UserId,
            /* 2*/ AggMessages::PartyId,
            /* 3*/ AggMessages::Nickname,
            /* 4*/ AggMessages::Username,
            /* 5*/ AggMessages::Discriminator,
            /* 6*/ AggMessages::UserFlags,
            /* 7*/ AggMessages::MentionIds,
            /* 8*/ AggMessages::MentionKinds,
            /* 9*/ AggMessages::EditedAt,
            /*10*/ AggMessages::MessageFlags,
            /*11*/ AggMessages::Content,
            /*12*/ AggMessages::RoleIds,
            /*13*/ AggMessages::AvatarId,
            /*14*/ AggMessages::AttachmentMeta,
            /*15*/ AggMessages::AttachmentPreview,
        ])
        .and_where(AggMessages::RoomId.equals(room_id_var.clone()))
        .and_where(
            // test if message is deleted
            AggMessages::MessageFlags
                .bit_and(Literal::Int2(MessageFlags::DELETED.bits()))
                .equals(Literal::Int2(0)),
        );

    match mode {
        MessageSearch::Before(_) | MessageSearch::After(_) => {
            query = query.from_table::<AggMessages>().limit(limit_var.clone())
        }
        _ => {}
    }

    query = match mode {
        MessageSearch::After(_) => query
            .and_where(AggMessages::MsgId.greater_than(msg_id_var))
            .order_by(AggMessages::MsgId.ascending()),
        MessageSearch::Before(_) => query
            .and_where(AggMessages::MsgId.less_than(msg_id_var))
            .order_by(AggMessages::MsgId.descending()),

        MessageSearch::Around(_) => {
            tables! {
                struct NumberedMsgs {
                    MsgId: Messages::Id,
                    RowNumber: Type::INT8,
                }

                struct Offsets {
                    Offset: Type::INT8,
                }
            }

            // generate a sequence of message ids with their row number
            let numbered = NumberedMsgs::as_query(
                Query::select()
                    .from_table::<AggMessages>()
                    .expr(AggMessages::MsgId.alias_to(NumberedMsgs::MsgId))
                    .expr(
                        Builtin::row_number(())
                            .over(AggMessages::MsgId.ascending())
                            .alias_to(NumberedMsgs::RowNumber),
                    ),
            );

            // generate a set of offsets around 0
            let offsets = Offsets::as_query(
                Query::select()
                    .expr(
                        Builtin::generate_series((limit_var.clone().neg(), Literal::Int8(-1)))
                            .alias_to(Offsets::Offset),
                    )
                    .union_all(
                        Query::select().expr(
                            Builtin::generate_series((Literal::Int8(1), limit_var.clone()))
                                .alias_to(Offsets::Offset),
                        ),
                    ),
            );

            query = query
                .with(numbered.exclude())
                .with(offsets.exclude())
                .from(
                    AggMessages::inner_join_table::<NumberedMsgs>()
                        .on(AggMessages::MsgId.equals(NumberedMsgs::MsgId)),
                )
                .and_where(
                    // select only messages in the offset range
                    NumberedMsgs::RowNumber.in_query(
                        Query::select()
                            .expr(NumberedMsgs::RowNumber.add(Offsets::Offset))
                            .from(NumberedMsgs::cross_join_table::<Offsets>())
                            .and_where(NumberedMsgs::MsgId.equals(msg_id_var)),
                    ),
                )
                .order_by(AggMessages::MsgId.ascending());

            query
        }
    };

    if check_perms {
        tables! {
            struct AggPerm {
                Perms: AggRoomPerms::Perms,
            }
        }

        const READ_MESSAGE: i64 = Permission {
            party: PartyPermissions::empty(),
            room: RoomPermissions::READ_MESSAGES,
            stream: StreamPermissions::empty(),
        }
        .pack() as i64;

        query = query
            .with(AggPerm::as_query(
                Query::select()
                    .expr(AggRoomPerms::Perms.alias_to(AggPerm::Perms))
                    .from_table::<AggRoomPerms>()
                    .and_where(AggRoomPerms::UserId.equals(user_id_var))
                    .and_where(AggRoomPerms::RoomId.equals(room_id_var)),
            ))
            .and_where(
                AggPerm::Perms
                    .bit_and(Literal::Int8(READ_MESSAGE))
                    .equals(Literal::Int8(READ_MESSAGE)),
            )
    }

    query
}
