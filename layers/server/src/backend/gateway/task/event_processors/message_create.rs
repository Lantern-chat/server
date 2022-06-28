use thorn::pg::Json;

use crate::backend::{
    gateway::Event,
    util::encrypted_asset::{encrypt_snowflake, encrypt_snowflake_opt},
};

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
            AggMessages::ProfileBits,
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

    let ext_party_id = row.try_get(Columns::party_id())?;

    if party_id != ext_party_id {
        log::warn!("Message PartyID from event-log and PartyID from Message differ!");
    }

    let room_id = row.try_get(Columns::room_id())?;

    let mut msg = Message {
        id,
        party_id: ext_party_id,
        created_at: id.timestamp(),
        room_id,
        flags: MessageFlags::from_bits_truncate(row.try_get(Columns::message_flags())?),
        kind: MessageKind::try_from(row.try_get::<_, i16>(Columns::kind())?).unwrap_or_default(),
        edited_at: row.try_get(Columns::edited_at())?,
        content: row.try_get(Columns::content())?,
        author: User {
            id: row.try_get(Columns::user_id())?,
            username: row.try_get(Columns::username())?,
            discriminator: row.try_get(Columns::discriminator())?,
            flags: UserFlags::from_bits_truncate(row.try_get(Columns::user_flags())?).publicize(),
            email: None,
            preferences: None,
            profile: match row.try_get(Columns::profile_bits())? {
                None => None,
                Some(bits) => Some(UserProfile {
                    bits,
                    avatar: encrypt_snowflake_opt(state, row.try_get(Columns::avatar_id())?),
                    banner: None,
                    status: None,
                    bio: None,
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
        thread_id: None,
        user_mentions: Vec::new(),
        role_mentions: Vec::new(),
        room_mentions: Vec::new(),
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
