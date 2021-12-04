use std::borrow::Cow;

use futures::FutureExt;
use schema::{Snowflake, SnowflakeExt};
use smol_str::SmolStr;

use crate::{
    ctrl::{auth::Authorization, perm::get_cached_room_permissions, Error, SearchMode},
    ServerState,
};

use models::*;

pub mod embed;
pub mod slash;

#[derive(Debug, Deserialize)]
pub struct CreateMessageForm {
    content: SmolStr,

    #[serde(default)]
    attachments: Vec<Snowflake>,
}

pub async fn create_message(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    form: CreateMessageForm,
) -> Result<Option<Message>, Error> {
    // fast-path for if the perm_cache does contain a value, otherwise defer until content is checked
    let perm = match state.perm_cache.get(auth.user_id, room_id).await {
        Some(perm) => {
            if !perm.perm.room.contains(RoomPermissions::SEND_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            Some(perm.perm)
        }
        None => None,
    };

    let mut trimmed_content = Cow::Borrowed(form.content.trim());

    // if empty but not containing attachments
    if trimmed_content.is_empty() && form.attachments.is_empty() {
        return Err(Error::BadRequest);
    }

    if !trimmed_content.is_empty() {
        let mut trimming = false;
        let mut new_content = String::new();
        let mut count = 0;

        // TODO: Don't strip newlines inside code blocks?
        for (idx, &byte) in trimmed_content.as_bytes().iter().enumerate() {
            // if we encounted a newline
            if byte == b'\n' {
                count += 1; // count up

                // if over 2 consecutive newlines, begin trimming
                if count > 2 {
                    // if not already trimming, push everything tested up until this point
                    // notably not including the third newline
                    if !trimming {
                        trimming = true;
                        new_content.push_str(&trimmed_content[..idx]);
                    }

                    // skip any additional newlines
                    continue;
                }
            } else {
                // reset count if newline streak broken
                count = 0;
            }

            // if trimming, push the new byte
            if trimming {
                unsafe { new_content.as_mut_vec().push(byte) };
            }
        }

        if trimming {
            trimmed_content = Cow::Owned(new_content);
        }

        let newlines = bytecount::count(trimmed_content.as_bytes(), b'\n');

        let too_large = !state.config.message_len.contains(&trimmed_content.len());
        let too_long = newlines > state.config.max_newlines;

        if too_large || too_long {
            return Err(Error::BadRequest);
        }
    }

    // since acquiring the database connection may be expensive,
    // defer it until we need it, such as if the permissions cache didn't have a value
    let mut maybe_db = None;

    let perm = match perm {
        Some(perm) => perm,
        None => {
            let db = state.db.write.get().await?;

            let perm = crate::ctrl::perm::get_room_permissions(&db, auth.user_id, room_id).await?;

            if !perm.room.contains(RoomPermissions::SEND_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            maybe_db = Some(db);

            perm
        }
    };

    let msg_id = Snowflake::now();

    // check this before acquiring database connection
    if !form.attachments.is_empty() && !perm.room.contains(RoomPermissions::ATTACH_FILES) {
        return Err(Error::Unauthorized);
    }

    // modify content before inserting it into the database
    let modified_content = match slash::process_slash(&trimmed_content) {
        Ok(Some(content)) => content,
        Ok(None) => return Ok(None),
        Err(e) => return Err(e),
    };

    // message is good to go, so fire off the embed processing
    if !modified_content.is_empty() && perm.room.contains(RoomPermissions::EMBED_LINKS) {
        embed::process_embeds(msg_id, &modified_content);
    }

    // if we avoided getting a database connection until now, do it now
    let db = match maybe_db {
        Some(db) => db,
        None => state.db.write.get().await?,
    };

    let res = if !form.attachments.is_empty() {
        create_message_full(db, state, auth, room_id, msg_id, &form, &modified_content)
            .boxed()
            .await
    } else {
        do_send_simple_message(db, state, auth.user_id, room_id, msg_id, &modified_content).await
    };

    res.map(Option::Some)
}

pub async fn do_send_simple_message(
    db: db::pool::Object,
    state: ServerState,
    user_id: Snowflake,
    room_id: Snowflake,
    msg_id: Snowflake,
    content: &str,
) -> Result<Message, Error> {
    let row = db
        .query_opt_cached_typed(|| query(), &[&user_id, &room_id, &msg_id, &content])
        .await?;

    let row = match row {
        None => return Err(Error::NotFound),
        Some(row) => row,
    };

    let party_id: Option<Snowflake> = row.try_get(0)?;
    let nickname: Option<SmolStr> = row.try_get(1)?;
    let roles = row.try_get(2)?;

    Ok(Message {
        id: msg_id,
        party_id,
        room_id,
        member: nickname.map(|nick| PartyMember {
            user: None,
            nick: Some(nick),
            roles,
            presence: None,
        }),
        author: User {
            id: user_id,
            username: row.try_get(3)?,
            discriminator: row.try_get(4)?,
            flags: UserFlags::from_bits_truncate(row.try_get(5)?).publicize(),
            avatar: None,
            status: row.try_get(6)?,
            bio: row.try_get(7)?,
            email: None,
            preferences: None,
        },
        thread_id: None,
        created_at: msg_id.timestamp(),
        edited_at: None,
        flags: MessageFlags::empty(),
        content: content.into(),
        user_mentions: Vec::new(),
        role_mentions: Vec::new(),
        room_mentions: Vec::new(),
        reactions: Vec::new(),
        attachments: Vec::new(),
        embeds: Vec::new(),
    })
}

use thorn::*;

fn query() -> impl AnyQuery {
    use schema::*;

    tables! {
        struct AggMsg {
            UserId: Users::Id,
            RoomId: Rooms::Id,
        }
    }

    let user_id_var = Var::at(Users::Id, 1);
    let room_id_var = Var::at(Rooms::Id, 2);
    let msg_id_var = Var::at(Messages::Id, 3);
    let content_var = Var::at(Messages::Content, 4);

    let insert = AggMsg::as_query(
        Query::insert()
            .into::<Messages>()
            .cols(&[
                Messages::Id,
                Messages::UserId,
                Messages::RoomId,
                Messages::Content,
            ])
            .values(vec![msg_id_var, user_id_var, room_id_var, content_var])
            .returning(Messages::UserId.alias_to(AggMsg::UserId))
            .returning(Messages::RoomId.alias_to(AggMsg::RoomId)),
    );

    let roles = Query::select()
        .expr(Builtin::array_agg(RoleMembers::RoleId))
        .from(RoleMembers::inner_join_table::<Roles>().on(RoleMembers::RoleId.equals(Roles::Id)))
        .and_where(RoleMembers::UserId.equals(AggMsg::UserId))
        .and_where(Roles::PartyId.equals(Party::Id));

    Query::with()
        .with(insert)
        .select()
        .col(Party::Id)
        .col(PartyMember::Nickname)
        .expr(roles.as_value())
        .cols(&[
            Users::Username,
            Users::Discriminator,
            Users::Flags,
            Users::CustomStatus,
            Users::Biography,
        ])
        .from_table::<Users>()
        .from(
            Rooms::left_join_table::<Party>()
                .on(Party::Id.equals(Rooms::PartyId))
                .left_join_table::<PartyMember>()
                .on(PartyMember::PartyId.equals(Party::Id)),
        )
        .and_where(Rooms::Id.equals(AggMsg::RoomId))
        .and_where(Users::Id.equals(AggMsg::UserId))
        .and_where(PartyMember::UserId.equals(AggMsg::UserId))
}

pub async fn create_message_full(
    mut db: db::pool::Object,
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    form: &CreateMessageForm,
    trimmed: &str,
) -> Result<Message, Error> {
    let t = db.transaction().await?;

    let msg_future = async {
        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::insert()
                    .into::<Messages>()
                    .cols(&[
                        Messages::Id,
                        Messages::UserId,
                        Messages::RoomId,
                        Messages::Content,
                    ])
                    .values(vec![
                        Var::of(Messages::Id),
                        Var::of(Messages::UserId),
                        Var::of(Messages::RoomId),
                        Var::of(Messages::Content),
                    ])
            },
            &[&msg_id, &auth.user_id, &room_id, &trimmed],
        )
        .await?;

        Ok(())
    };

    let attachment_future = async {
        t.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                tables! {
                    struct AggIds {
                        Id: Files::Id,
                    }
                }

                let msg_id = Var::at(Messages::Id, 1);
                let att_id = Var::at(SNOWFLAKE_ARRAY, 2);

                Query::with()
                    .with(
                        AggIds::as_query(
                            Query::select().expr(Call::custom("UNNEST").arg(att_id).alias_to(AggIds::Id)),
                        )
                        .exclude(),
                    )
                    .insert()
                    .into::<Attachments>()
                    .cols(&[Attachments::FileId, Attachments::MessageId])
                    .query(
                        Query::select()
                            .col(AggIds::Id)
                            .expr(msg_id)
                            .from_table::<AggIds>()
                            .as_value(),
                    )
            },
            &[&msg_id, &form.attachments],
        )
        .await?;

        Ok(())
    };

    let get_message_future = async {
        let row = t
            .query_one_cached_typed(|| super::get_one::get_one_without_perms(), &[&room_id, &msg_id])
            .await?;

        super::get_one::parse_msg(&state, room_id, msg_id, row)
    };

    let (_, _, msg) = tokio::try_join!(msg_future, attachment_future, get_message_future)?;

    t.commit().await?;

    Ok(msg)
}
