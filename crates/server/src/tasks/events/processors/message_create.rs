use crate::web::gateway::{msg::ServerMsg, Event};

use super::*;

pub async fn message_create(
    state: &ServerState,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let db = state.db.read.get().await?;

    let row = db
        .query_one_cached_typed(
            || {
                use db::schema::*;

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
                        /*11*/ AggMessages::Roles,
                    ])
            },
            &[&id],
        )
        .await?;

    let ext_party_id = row.try_get(1)?;

    if party_id != ext_party_id {
        log::warn!("Message PartyID from event-log and PartyID from Message differ!");
    }

    let mut msg = Message {
        id,
        party_id: ext_party_id,
        created_at: time::PrimitiveDateTime::from(id.timestamp())
            .assume_utc()
            .format(time::Format::Rfc3339),
        room_id: row.try_get(2)?,
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
            avatar_id: None,
        },
        member: match party_id {
            None => None,
            Some(_) => Some(PartyMember {
                user: None,
                nick: row.try_get(3)?,
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
            .broadcast_event(Event::new(event)?, party_id, false)
            .await;
    }

    Ok(())
}
