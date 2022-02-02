use std::borrow::Cow;

use hashbrown::HashSet;
use sdk::models::*;
use smol_str::SmolStr;
use thorn::pg::Json;

use crate::{
    ctrl::{util::encrypted_asset::encrypt_snowflake_opt, Error},
    permission_cache::PermMute,
    web::auth::Authorization,
    ServerState,
};

#[derive(Deserialize)]
pub struct EditMessageForm {
    #[serde(default)]
    content: SmolStr,

    #[serde(default)]
    attachments: Vec<Snowflake>,
}

pub async fn edit_message(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    form: EditMessageForm,
) -> Result<Option<Message>, Error> {
    // fast-path for if the perm_cache does contain a value
    let perm = match state.perm_cache.get(auth.user_id, room_id).await {
        Some(PermMute { perm, .. }) => {
            // Mostly same rules as creating messages, as they are sending new content
            if !perm.room.contains(RoomPermissions::SEND_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            Some(perm)
        }
        None => None,
    };

    let trimmed_content = form.content.trim();

    // early reject if empty but not containing attachments
    if trimmed_content.is_empty() && form.attachments.is_empty() {
        // TODO: Edit a mesage to have zero anything, should it be deleted instead?
        return Err(Error::BadRequest);
    }

    let mut db = state.db.write.get().await?;

    let perm = match perm {
        Some(perm) => perm,
        None => {
            let perm = crate::ctrl::perm::get_room_permissions(&db, auth.user_id, room_id).await?;

            if !perm.room.contains(RoomPermissions::SEND_MESSAGES) {
                return Err(Error::Unauthorized);
            }

            perm
        }
    };

    // first read-only query is not within the transaction because without repeatable-read it doesn't matter anyway
    let prev = db
        .query_opt_cached_typed(|| query_existing_message(), &[&msg_id, &room_id])
        .await?;

    let row = match prev {
        Some(row) => row,
        None => return Err(Error::NotFound),
    };

    let author_id: Snowflake = row.try_get(0)?;

    if author_id != auth.user_id {
        return Err(Error::Unauthorized);
    }

    let _prev_flags = MessageFlags::from_bits_truncate(row.try_get(1)?);
    let prev_content: Option<&str> = row.try_get(2)?;
    let prev_files: Option<Vec<Snowflake>> = row.try_get(3)?;

    // do full trimming after possible error locations
    let trimmed_content = super::create::trim::trim_message(&state, &trimmed_content)?;

    // edits cannot perform actions, but are subject to replacements
    let modified_content = match super::create::slash::process_slash(&trimmed_content, false) {
        Ok(Some(content)) => content,
        Ok(None) => return Ok(None),
        Err(e) => return Err(e),
    };

    if !modified_content.is_empty() && perm.room.contains(RoomPermissions::EMBED_LINKS) {
        // TODO: Reprocess embeds
    }

    // this must go above the query futures creation to span their entire lexical lifetime
    let t = db.transaction().await?;

    // queue up queries to be pipelined
    use futures::future::{ok, Either};
    let mut add_attachments = Either::Left(ok::<(), Error>(()));
    let mut orphan_attachments = Either::Left(ok::<(), Error>(()));
    let mut update_message = Either::Left(ok::<(), Error>(()));

    // if there are old or new attachments
    if !prev_files.is_none() || !form.attachments.is_empty() {
        // attachments may be unordered, so a Set is required
        let pre_set: HashSet<Snowflake> = HashSet::from_iter(prev_files.unwrap_or_default());
        let new_set: HashSet<Snowflake> = HashSet::from_iter(form.attachments);

        if pre_set != new_set {
            let added = new_set.difference(&pre_set).copied().collect::<Vec<_>>();

            if !added.is_empty() && !perm.room.contains(RoomPermissions::EDIT_NEW_ATTACHMENT) {
                return Err(Error::Unauthorized);
            }

            let removed = pre_set.difference(&new_set).copied().collect::<Vec<_>>();

            let t = &t; // hackery, can't take ownership of t within below async move blocks, so move a reference

            if !added.is_empty() {
                add_attachments =
                    Either::Right(async move {
                        // TODO: Deduplicate this with query in message_create
                        t.execute_cached_typed(
                            || {
                                use schema::*;
                                use thorn::*;

                                tables! {
                                    struct AggIds {
                                        Id: Files::Id,
                                    }
                                }

                                let msg_id_var = Var::at(Messages::Id, 1);
                                let att_id_var = Var::at(SNOWFLAKE_ARRAY, 2);

                                Query::with()
                                    .with(
                                        AggIds::as_query(Query::select().expr(
                                            Call::custom("UNNEST").arg(att_id_var).alias_to(AggIds::Id),
                                        ))
                                        .exclude(),
                                    )
                                    .insert()
                                    .into::<Attachments>()
                                    .cols(&[Attachments::FileId, Attachments::MessageId])
                                    .query(
                                        Query::select()
                                            .col(AggIds::Id)
                                            .expr(msg_id_var)
                                            .from_table::<AggIds>()
                                            .as_value(),
                                    )
                            },
                            &[&msg_id, &added],
                        )
                        .await?;

                        Ok(())
                    });
            }

            if !removed.is_empty() {
                orphan_attachments = Either::Right(async move {
                    t.execute_cached_typed(
                        || {
                            use schema::*;
                            use thorn::*;

                            Query::update()
                                .table::<Attachments>()
                                .set(
                                    Attachments::Flags,
                                    Attachments::Flags
                                        .bit_or(Literal::Int2(flags::AttachmentFlags::ORPHANED.bits())),
                                )
                                .and_where(Attachments::FileId.equals(Builtin::any(Var::of(SNOWFLAKE_ARRAY))))
                        },
                        &[&removed],
                    )
                    .await?;

                    Ok(())
                });
            }
        }
    }

    match (prev_content, modified_content.as_ref()) {
        (Some(prev), modified) if prev == modified => {}
        (None, "") => {}
        _ => {
            let content = if modified_content.is_empty() { None } else { Some(modified_content.as_ref()) };

            let t = &t;

            update_message = Either::Right(async move {
                t.execute_cached_typed(
                    || {
                        use schema::*;
                        use thorn::*;

                        let msg_id_var = Var::at(Messages::Id, 1);
                        let msg_content_var = Var::at(Messages::Content, 2);

                        Query::update()
                            .table::<Messages>()
                            .and_where(Messages::Id.equals(msg_id_var))
                            .set(Messages::Content, msg_content_var)
                            .set(Messages::EditedAt, Builtin::now(()))
                    },
                    &[&msg_id, &content],
                )
                .await?;

                Ok(())
            })
        }
    }

    tokio::try_join!(add_attachments, orphan_attachments, update_message)?;

    let msg = {
        let row = t
            .query_one_cached_typed(|| super::get_one::get_one_without_perms(), &[&room_id, &msg_id])
            .await?;

        super::get_one::parse_msg(&state, &row)?
    };

    t.commit().await?;

    Ok(Some(msg))
}

fn query_existing_message() -> impl thorn::AnyQuery {
    use schema::*;
    use thorn::*;

    tables! {
        struct AggFileIds {
            FileIds: SNOWFLAKE_ARRAY
        }
    }

    Query::select()
        .and_where(Messages::Id.equals(Var::of(Messages::Id)))
        .and_where(Messages::RoomId.equals(Var::of(Messages::RoomId)))
        .cols(&[Messages::UserId, Messages::Flags, Messages::Content])
        .col(AggFileIds::FileIds)
        .from(
            // Use a lateral join because it's easier to do aggregate this way
            Messages::left_join(Lateral(AggFileIds::as_query(
                Query::select()
                    .from_table::<Attachments>()
                    .expr(Builtin::array_agg(Attachments::FileId).alias_to(AggFileIds::FileIds))
                    .and_where(Attachments::MessageId.equals(Messages::Id)),
            )))
            .on(Literal::TRUE),
        )
        .and_where(
            Messages::Flags
                .bit_and(Literal::Int2(MessageFlags::DELETED.bits()))
                .equals(Literal::Int2(0)),
        )
}
