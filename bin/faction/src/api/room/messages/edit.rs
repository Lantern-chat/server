use hashbrown::HashSet;

use sdk::models::*;

use crate::{prelude::*, state::permission_cache::PermMute};

use sdk::api::commands::room::EditMessageBody;

pub async fn edit_message(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    body: &Archived<EditMessageBody>,
) -> Result<Option<Message>, Error> {
    // fast-path for if the perm_cache does contain a value
    let perms = match state.perm_cache.get(auth.user_id(), room_id).await {
        Some(PermMute { perms, .. }) => {
            // Mostly same rules as creating messages, as they are sending new content
            if !perms.contains(Permissions::SEND_MESSAGES) {
                return err(CommonError::Unauthorized);
            }

            Some(perms)
        }
        None => None,
    };

    let trimmed_content = body.content.trim();

    // early reject if empty but not containing attachments
    if trimmed_content.is_empty() && body.attachments.is_empty() {
        // TODO: Edit a mesage to have zero anything, should it be deleted instead?
        return err(CommonError::BadRequest);
    }

    let mut db = state.db.write.get().await?;

    let perms = match perms {
        Some(perm) => perm,
        None => {
            let perms = crate::api::perm::get_room_permissions(&db, auth.user_id(), room_id).await?;

            if !perms.contains(Permissions::SEND_MESSAGES) {
                return err(CommonError::Unauthorized);
            }

            perms
        }
    };

    // first read-only query is not within the transaction because without repeatable-read it doesn't matter anyway
    let prev = db.query_opt_cached_typed(|| query_existing_message(), &[&msg_id, &room_id]).await?;

    let Some(row) = prev else {
        return err(CommonError::NotFound);
    };

    let author_id: Snowflake = row.try_get(0)?;

    if author_id != auth.user_id() {
        return err(CommonError::Unauthorized);
    }

    let _prev_flags = MessageFlags::from_bits_truncate_public(row.try_get(1)?);
    let prev_content: Option<&str> = row.try_get(2)?;
    let prev_files: Option<Vec<Snowflake>> = row.try_get(3)?;

    // do full trimming
    let Some(trimmed_content) = ({
        let config = state.config();
        md_utils::trim_message(
            trimmed_content,
            Some(md_utils::TrimLimits {
                len: config.shared.message_length.clone(),
                max_newlines: config.shared.max_newlines as usize,
            }),
        )
    }) else {
        return err(CommonError::BadRequest);
    };

    // edits cannot perform actions, but are subject to replacements
    let modified_content = match super::create::slash::process_slash(&trimmed_content, false) {
        Ok(Some(content)) => content,
        Ok(None) => return Ok(None),
        Err(e) => return Err(e),
    };

    if !modified_content.is_empty() && perms.contains(Permissions::EMBED_LINKS) {
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
    if prev_files.is_some() || !body.attachments.is_empty() {
        // attachments may be unordered, so a Set is required
        let pre_set: HashSet<Snowflake> = HashSet::from_iter(prev_files.unwrap_or_default());
        let new_set: HashSet<Snowflake> = HashSet::from_iter(body.attachments.as_slice().iter().copied());

        if pre_set != new_set {
            let added = new_set.difference(&pre_set).copied().collect::<Vec<_>>();

            if !added.is_empty() && !perms.contains(Permissions::EDIT_NEW_ATTACHMENT) {
                return err(CommonError::Unauthorized);
            }

            let removed = pre_set.difference(&new_set).copied().collect::<Vec<_>>();

            let t = &t; // hackery, can't take ownership of t within below async move blocks, so move a reference

            if !added.is_empty() {
                add_attachments = Either::Right(async move {
                    t.execute_cached_typed(|| add_attachments_query(), &[&msg_id, &added]).await?;

                    Ok(())
                });
            }

            if !removed.is_empty() {
                orphan_attachments = Either::Right(async move {
                    t.execute_cached_typed(|| orphan_attachments_query(), &[&removed]).await?;

                    Ok(())
                });
            }
        }
    }

    // avoid reprocessing the message content if it's identical
    if prev_content.unwrap_or("") != modified_content {
        update_message = Either::Right(async {
            #[rustfmt::skip]
            t.execute2(schema::sql! {
                UPDATE Messages SET (Content, EditedAt) = (NULLIF(#{&modified_content as Messages::Content}, ""), NOW())
                 WHERE Messages.Id = #{&msg_id as Messages::Id}
            }).await?;

            Ok(())
        });
    }

    tokio::try_join!(add_attachments, orphan_attachments, update_message)?;

    let msg = super::get::get_one(state, &t, msg_id).await?;

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
                    .and_where(Attachments::MsgId.equals(Messages::Id)),
            )))
            .on(true.lit()),
        )
        .and_where(Messages::Flags.has_no_bits(MessageFlags::DELETED.bits().lit()))
}

// TODO: Deduplicate this with query in message_create
fn add_attachments_query() -> impl thorn::AnyQuery {
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
            AggIds::as_query(Query::select().expr(Builtin::unnest((att_id_var,)).alias_to(AggIds::Id))).exclude(),
        )
        .insert()
        .into::<Attachments>()
        .cols(&[Attachments::FileId, Attachments::MsgId])
        .query(Query::select().col(AggIds::Id).expr(msg_id_var).from_table::<AggIds>().as_value())
}

fn orphan_attachments_query() -> impl thorn::AnyQuery {
    use schema::*;
    use thorn::*;

    Query::update()
        .table::<Attachments>()
        .set(
            Attachments::Flags,
            Attachments::Flags.bitor(flags::AttachmentFlags::ORPHANED.bits().lit()),
        )
        .and_where(Attachments::FileId.equals(Builtin::any(Var::of(SNOWFLAKE_ARRAY))))
}
