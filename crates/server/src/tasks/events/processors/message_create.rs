use crate::{
    ctrl::util::encrypted_asset::encrypt_snowflake,
    web::gateway::{msg::ServerMsg, Event},
};

use super::*;

pub async fn message_create(
    state: &ServerState,
    db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let row = db
        .query_one_cached_typed(
            || {
                use schema::*;

                Query::select()
                    .from_table::<AggMessages>()
                    .and_where(AggMessages::MsgId.equals(Var::of(AggMessages::MsgId)))
                    .cols(&[
                        /* 0*/ AggMessages::UserId,
                        /* 1*/ AggMessages::PartyId,
                        /* 2*/ AggMessages::RoomId,
                        /* 3*/ AggMessages::Nickname,
                        /* 4*/ AggMessages::Username,
                        /* 5*/ AggMessages::Discriminator,
                        /* 6*/ AggMessages::UserFlags,
                        /* 7*/ AggMessages::MentionIds,
                        /* 8*/ AggMessages::MentionKinds,
                        /* 9*/ AggMessages::MessageFlags,
                        /*10*/ AggMessages::Content,
                        /*11*/ AggMessages::RoleIds,
                        /*12*/ AggMessages::AttachmentMeta,
                        /*13*/ AggMessages::AttachmentPreview,
                        /*14*/ AggMessages::AvatarId,
                    ])
            },
            &[&id],
        )
        .await?;

    let ext_party_id = row.try_get(1)?;

    if party_id != ext_party_id {
        log::warn!("Message PartyID from event-log and PartyID from Message differ!");
    }

    let room_id = row.try_get(2)?;

    let mut msg = Message {
        id,
        party_id: ext_party_id,
        created_at: id.format_timestamp(),
        room_id,
        flags: MessageFlags::from_bits_truncate(row.try_get(9)?),
        edited_at: None, // new message, not edited
        content: row.try_get(10)?,
        author: User {
            id: row.try_get(0)?,
            username: row.try_get(4)?,
            discriminator: row.try_get(5)?,
            flags: UserFlags::from_bits_truncate(row.try_get(6)?).publicize(),
            status: None,
            bio: None,
            email: None,
            preferences: None,
            avatar: match row.try_get(14)? {
                Some(avatar_id) => Some(encrypt_snowflake(&state, avatar_id)),
                None => None,
            },
        },
        member: match party_id {
            None => None,
            Some(_) => Some(PartyMember {
                user: None,
                nick: row.try_get(3)?,
                roles: row.try_get(11)?,
                presence: None,
            }),
        },
        thread_id: None,
        user_mentions: Vec::new(),
        role_mentions: Vec::new(),
        room_mentions: Vec::new(),
        attachments: {
            let mut attachments = Vec::new();

            let meta: Option<serde_json::Value> = row.try_get(12)?;

            if let Some(meta) = meta {
                let meta: Vec<schema::AggAttachmentsMeta> = serde_json::from_value(meta)?;
                let previews: Vec<Option<Vec<u8>>> = row.try_get(13)?;

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
                            preview: preview.map(|p| p.to_z85().unwrap().into()),
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

    if let Some(party_id) = msg.party_id {
        let event = ServerMsg::new_messagecreate(msg);

        state
            .gateway
            .broadcast_event(Event::new(event, Some(room_id))?, party_id, false)
            .await;
    }

    Ok(())
}
