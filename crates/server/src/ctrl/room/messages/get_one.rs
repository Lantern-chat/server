use db::Snowflake;

use models::*;

use crate::{ctrl::Error, web::auth::Authorization, ServerState};

pub async fn get_one(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> Result<Message, Error> {
    let had_perms = if let Some(perm) = state.perm_cache.get(auth.user_id, room_id).await {
        if !perm.perm.room.contains(RoomPermissions::READ_MESSAGES) {
            return Err(Error::NotFound);
        }

        true
    } else {
        false
    };

    let db = state.db.read.get().await?;

    let row = if had_perms {
        db.query_opt_cached_typed(|| get_one_without_perms(), &[&room_id, &msg_id])
            .await?
    } else {
        db.query_opt_cached_typed(|| get_one_with_perms(), &[&auth.user_id, &room_id, &msg_id])
            .await?
    };

    let row = match row {
        None => return Err(Error::NotFound),
        Some(row) => row,
    };

    let party_id: Option<Snowflake> = row.try_get(1)?;

    let mut msg = Message {
        id: msg_id,
        party_id,
        created_at: msg_id.format_timestamp(),
        room_id,
        flags: MessageFlags::from_bits_truncate(row.try_get(9)?),
        edited_at: row
            .try_get::<_, Option<chrono::NaiveDateTime>>(8)?
            .map(crate::util::time::format_naivedatetime),
        content: row.try_get(10)?,
        author: User {
            id: row.try_get(0)?,
            username: row.try_get(3)?,
            discriminator: row.try_get(4)?,
            flags: UserFlags::from_bits_truncate(row.try_get(5)?).publicize(),
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
                nick: row.try_get(2)?,
                roles: row.try_get(11)?,
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

    let mention_kinds: Option<Vec<i32>> = row.try_get(7)?;
    if let Some(mention_kinds) = mention_kinds {
        // lazily parse ids
        let mention_ids: Vec<Snowflake> = row.try_get(6)?;

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

use thorn::*;

mod consts {
    use db::schema::*;

    pub const COLUMNS: &[AggMessages] = &[
        /* 0*/ AggMessages::UserId,
        /* 1*/ AggMessages::PartyId,
        /* 2*/ AggMessages::Nickname,
        /* 3*/ AggMessages::Username,
        /* 4*/ AggMessages::Discriminator,
        /* 5*/ AggMessages::UserFlags,
        /* 6*/ AggMessages::MentionIds,
        /* 7*/ AggMessages::MentionKinds,
        /* 8*/ AggMessages::EditedAt,
        /* 9*/ AggMessages::MessageFlags,
        /*10*/ AggMessages::Content,
        /*11*/ AggMessages::Roles,
    ];
}

fn get_one_without_perms() -> impl AnyQuery {
    use db::schema::*;

    Query::select()
        .from_table::<AggMessages>()
        .cols(consts::COLUMNS)
        .and_where(AggMessages::RoomId.equals(Var::of(Rooms::Id)))
        .and_where(AggMessages::MsgId.equals(Var::of(Messages::Id)))
        .and_where(
            // test if message is deleted
            AggMessages::MessageFlags
                .bit_and(Literal::Int2(MessageFlags::DELETED.bits()))
                .equals(Literal::Int2(0)),
        )
}

fn get_one_with_perms() -> impl AnyQuery {
    use db::schema::*;

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

    let user_id_var = Var::at(Users::Id, 1);
    let room_id_var = Var::at(Rooms::Id, 2);
    let msg_id_var = Var::at(Messages::Id, 3);

    let permissions = AggPerm::as_query(
        Query::select()
            .expr(AggRoomPerms::Perms.alias_to(AggPerm::Perms))
            .from_table::<AggRoomPerms>()
            .and_where(AggRoomPerms::UserId.equals(user_id_var.clone()))
            .and_where(AggRoomPerms::RoomId.equals(room_id_var.clone())),
    );

    Query::with()
        .with(permissions)
        .select()
        .and_where(
            AggPerm::Perms
                .bit_and(Literal::Int8(READ_MESSAGE))
                .equals(Literal::Int8(READ_MESSAGE)),
        )
        .from_table::<AggMessages>()
        .cols(consts::COLUMNS)
        .and_where(AggMessages::RoomId.equals(room_id_var))
        .and_where(AggMessages::MsgId.equals(msg_id_var))
        .and_where(
            // test if message is deleted
            AggMessages::MessageFlags
                .bit_and(Literal::Int2(MessageFlags::DELETED.bits()))
                .equals(Literal::Int2(0)),
        )
}
