use schema::Snowflake;

use sdk::models::*;
use thorn::pg::Json;

use crate::{backend::util::encrypted_asset::encrypt_snowflake_opt, Authorization, Error, ServerState};

pub async fn get_one(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> Result<Message, Error> {
    let had_perms = if let Some(perm) = state.perm_cache.get(auth.user_id, room_id).await {
        if !perm.contains(RoomPermissions::READ_MESSAGE_HISTORY) {
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

    match row {
        Some(row) => parse_msg(&state, &row),
        None => Err(Error::NotFound),
    }
}

pub(crate) fn parse_msg(state: &ServerState, row: &db::Row) -> Result<Message, Error> {
    let flags = MessageFlags::from_bits_truncate(row.try_get(Columns::message_flags())?);

    // doing this in the application layer results in simpler queries
    if flags.contains(MessageFlags::DELETED) {
        return Err(Error::NotFound);
    }

    let msg_id = row.try_get(Columns::msg_id())?;
    let room_id = row.try_get(Columns::room_id())?;
    let party_id = row.try_get(Columns::party_id())?;

    let mut msg = Message {
        id: msg_id,
        party_id,
        created_at: msg_id.timestamp(),
        room_id,
        flags,
        kind: MessageKind::try_from(row.try_get::<_, i16>(Columns::kind())?).unwrap_or_default(),
        edited_at: row.try_get::<_, Option<_>>(Columns::edited_at())?,
        content: row.try_get(Columns::content())?,
        author: User {
            id: row.try_get(Columns::user_id())?,
            username: row.try_get(Columns::username())?,
            discriminator: row.try_get(Columns::discriminator())?,
            flags: UserFlags::from_bits_truncate(row.try_get(Columns::user_flags())?).publicize(),
            email: None,
            preferences: None,
            profile: match row.try_get(Columns::profile_bits())? {
                None => Nullable::Null,
                Some(bits) => Nullable::Some(UserProfile {
                    bits,
                    avatar: encrypt_snowflake_opt(&state, row.try_get(Columns::avatar_id())?).into(),
                    banner: Nullable::Undefined,
                    status: Nullable::Undefined,
                    bio: Nullable::Undefined,
                }),
            },
        },
        member: match party_id {
            None => None,
            Some(_) => Some(PartyMember {
                user: None,
                nick: row.try_get(Columns::nickname())?,
                roles: row.try_get(Columns::role_ids())?,
                presence: None,
                flags: None,
            }),
        },
        thread_id: row.try_get(Columns::thread_id())?,
        user_mentions: Vec::new(), // TODO
        role_mentions: Vec::new(), // TODO
        room_mentions: Vec::new(), // TODO
        attachments: {
            let mut attachments = Vec::new();

            let meta: Option<Json<Vec<schema::AggAttachmentsMeta>>> =
                row.try_get(Columns::attachment_meta())?;

            if let Some(Json(meta)) = meta {
                let previews: Vec<Option<&[u8]>> = row.try_get(Columns::attachment_preview())?;

                if meta.len() != previews.len() {
                    return Err(Error::InternalErrorStatic("Meta != Previews length"));
                }

                attachments.reserve(meta.len());

                for (meta, preview) in meta.into_iter().zip(previews) {
                    use z85::ToZ85;

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
        embeds: Vec::new(),
        reactions: Vec::new(),
    };

    let mention_kinds: Option<Vec<i32>> = row.try_get(Columns::mention_kinds())?;
    if let Some(mention_kinds) = mention_kinds {
        // lazily parse ids
        let mention_ids: Vec<Snowflake> = row.try_get(Columns::mention_ids())?;

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

use schema::AggMessages;
thorn::indexed_columns! {
    pub enum Columns {
        AggMessages::MsgId,
        AggMessages::RoomId,
        AggMessages::PartyId,
        AggMessages::UserId,
        AggMessages::Discriminator,
        AggMessages::Kind,
        AggMessages::UserFlags,
        AggMessages::EditedAt,
        AggMessages::AvatarId,
        AggMessages::ProfileBits,
        AggMessages::ThreadId,
        AggMessages::MessageFlags,
        AggMessages::Username,
        AggMessages::Nickname,
        AggMessages::Content,
        AggMessages::RoleIds,
        AggMessages::MentionIds,
        AggMessages::MentionKinds,
        AggMessages::AttachmentMeta,
        AggMessages::AttachmentPreview,
    }
}

pub(crate) fn get_one_without_perms() -> impl thorn::AnyQuery {
    use schema::*;
    use thorn::*;

    Query::select()
        .from_table::<AggMessages>()
        .cols(Columns::default())
        .and_where(AggMessages::RoomId.equals(Var::of(Rooms::Id)))
        .and_where(AggMessages::MsgId.equals(Var::of(Messages::Id)))
}

fn get_one_with_perms() -> impl thorn::AnyQuery {
    use schema::*;
    use thorn::*;

    tables! {
        struct AggPerm {
            Perms: AggRoomPerms::Perms,
        }
    }

    const READ_MESSAGES: i64 = Permission::PACKED_READ_MESSAGE_HISTORY as i64;

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
                .bit_and(READ_MESSAGES.lit())
                .equals(READ_MESSAGES.lit()),
        )
        .from_table::<AggMessages>()
        .cols(Columns::default())
        .and_where(AggMessages::RoomId.equals(room_id_var))
        .and_where(AggMessages::MsgId.equals(msg_id_var))
}
