use thorn::pg::Json;

use crate::{ctrl::util::encrypted_asset::encrypt_snowflake, web::gateway::Event};

use sdk::models::gateway::message::ServerMsg;

use super::prelude::*;

pub async fn message_create(
    state: &ServerState,
    db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let msg = get_message(state, db, id, party_id).await?;

    if let Some(party_id) = msg.party_id {
        let room_id = msg.room_id;

        let event = ServerMsg::new_message_create(msg);

        state
            .gateway
            .broadcast_event(Event::new(event, Some(room_id))?, party_id)
            .await;
    }

    Ok(())
}

pub async fn get_message(
    state: &ServerState,
    db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<Message, Error> {
    use schema::AggMessages;

    thorn::indexed_columns! {
        pub enum Columns {
            AggMessages::UserId,
            AggMessages::PartyId,
            AggMessages::RoomId,
            AggMessages::Kind,
            AggMessages::Nickname,
            AggMessages::Username,
            AggMessages::Discriminator,
            AggMessages::UserFlags,
            AggMessages::MentionIds,
            AggMessages::MentionKinds,
            AggMessages::MessageFlags,
            AggMessages::Content,
            AggMessages::RoleIds,
            AggMessages::AttachmentMeta,
            AggMessages::AttachmentPreview,
            AggMessages::AvatarId,
            AggMessages::EditedAt
        }
    }

    let row = db
        .query_one_cached_typed(
            || {
                use schema::*;

                Query::select()
                    .from_table::<AggMessages>()
                    .and_where(AggMessages::MsgId.equals(Var::of(AggMessages::MsgId)))
                    .cols(Columns::default())
            },
            &[&id],
        )
        .await?;

    let ext_party_id = row.try_get(Columns::PartyId as usize)?;

    if party_id != ext_party_id {
        log::warn!("Message PartyID from event-log and PartyID from Message differ!");
    }

    let room_id = row.try_get(Columns::RoomId as usize)?;

    let mut msg = Message {
        id,
        party_id: ext_party_id,
        created_at: id.timestamp(),
        room_id,
        flags: MessageFlags::from_bits_truncate(row.try_get(Columns::MessageFlags as usize)?),
        kind: MessageKind::try_from(row.try_get::<_, i16>(Columns::Kind as usize)?).unwrap_or_default(),
        edited_at: row.try_get(Columns::EditedAt as usize)?,
        content: row.try_get(Columns::Content as usize)?,
        author: User {
            id: row.try_get(Columns::UserId as usize)?,
            username: row.try_get(Columns::Username as usize)?,
            discriminator: row.try_get(Columns::Discriminator as usize)?,
            flags: UserFlags::from_bits_truncate(row.try_get(Columns::UserFlags as usize)?).publicize(),
            status: None,
            bio: None,
            email: None,
            preferences: None,
            avatar: match row.try_get(Columns::AvatarId as usize)? {
                Some(avatar_id) => Some(encrypt_snowflake(state, avatar_id)),
                None => None,
            },
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
        thread_id: None,
        user_mentions: Vec::new(),
        role_mentions: Vec::new(),
        room_mentions: Vec::new(),
        attachments: {
            let mut attachments = Vec::new();

            let meta: Option<Json<Vec<schema::AggAttachmentsMeta>>> =
                row.try_get(Columns::AttachmentMeta as usize)?;

            if let Some(Json(meta)) = meta {
                let previews: Vec<Option<&[u8]>> = row.try_get(Columns::AttachmentPreview as usize)?;

                if meta.len() != previews.len() {
                    return Err(Error::InternalErrorStatic("Meta != Previews length"));
                }

                attachments.reserve(meta.len());

                for (meta, preview) in meta.into_iter().zip(previews) {
                    use blurhash::base85::ToZ85;

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
