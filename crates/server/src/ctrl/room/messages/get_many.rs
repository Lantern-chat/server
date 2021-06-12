use db::{Snowflake, SnowflakeExt};

use futures::{Stream, StreamExt};
use models::*;
use thorn::pg::ToSql;

use crate::{
    ctrl::{perm::get_cached_room_permissions_with_conn, Error},
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

    let stream = if had_perms {
        let params: &[&(dyn ToSql + Sync)] = &[&room_id, &msg_id, &limit];

        match form.query {
            None | Some(MessageSearch::Before(_)) => db
                .query_stream_cached_typed(|| query(MessageSearch::Before(Snowflake::null()), false), params)
                .await?
                .boxed(),
            Some(MessageSearch::After(_)) => db
                .query_stream_cached_typed(|| query(MessageSearch::After(Snowflake::null()), false), params)
                .await?
                .boxed(),
            Some(MessageSearch::Around(_)) => db
                .query_stream_cached_typed(|| query(MessageSearch::Around(Snowflake::null()), false), params)
                .await?
                .boxed(),
        }
    } else {
        let params: &[&(dyn ToSql + Sync)] = &[&room_id, &msg_id, &limit, &auth.user_id];

        match form.query {
            None | Some(MessageSearch::Before(_)) => db
                .query_stream_cached_typed(|| query(MessageSearch::Before(Snowflake::null()), true), params)
                .await?
                .boxed(),
            Some(MessageSearch::After(_)) => db
                .query_stream_cached_typed(|| query(MessageSearch::After(Snowflake::null()), true), params)
                .await?
                .boxed(),
            Some(MessageSearch::Around(_)) => db
                .query_stream_cached_typed(|| query(MessageSearch::Around(Snowflake::null()), true), params)
                .await?
                .boxed(),
        }
    };

    Ok(stream.map(move |row| match row {
        Err(e) => Err(Error::from(e)),
        Ok(row) => {
            let party_id: Option<Snowflake> = row.try_get(2)?;
            let msg_id = row.try_get(0)?;

            let mut msg = Message {
                id: msg_id,
                party_id,
                created_at: time::PrimitiveDateTime::from(msg_id.timestamp())
                    .assume_utc()
                    .format(time::Format::Rfc3339),
                room_id,
                flags: MessageFlags::from_bits_truncate(row.try_get(10)?),
                edited_at: row
                    .try_get::<_, Option<time::PrimitiveDateTime>>(9)?
                    .map(|t| t.assume_utc().format(time::Format::Rfc3339)),
                content: row.try_get(11)?,
                author: User {
                    id: row.try_get(1)?,
                    username: row.try_get(4)?,
                    discriminator: row.try_get(5)?,
                    flags: UserFlags::from_bits_truncate(row.try_get(6)?).publicize(),
                    status: None,
                    bio: None,
                    email: None,
                    preferences: None,
                    avatar_id: None,
                },
                member: match party_id {
                    None => None,
                    Some(_) => Some(PartyMember {
                        user: None,
                        nick: row.try_get(3)?,
                        roles: row.try_get(12)?,
                    }),
                },
                thread_id: None,
                user_mentions: Vec::new(),
                role_mentions: Vec::new(),
                room_mentions: Vec::new(),
                attachments: Vec::new(),
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
    use db::schema::*;
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
            /*12*/ AggMessages::Roles,
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
