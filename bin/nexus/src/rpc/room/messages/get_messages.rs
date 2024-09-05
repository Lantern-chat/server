use crate::{internal::get_messages::GetMsgRequest, prelude::*};

use sdk::models::*;

use sdk::api::commands::room::GetMessages;

pub async fn get_many(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<GetMessages>,
) -> Result<impl Stream<Item = Result<Message, Error>> + '_, Error> {
    let room_id = cmd.room_id.into();
    let form = &cmd.body;

    let needs_perms = match state.perm_cache.get(auth.user_id(), room_id).await {
        None => true,
        Some(perms) => {
            if !perms.contains(Permissions::READ_MESSAGE_HISTORY) {
                return Err(Error::NotFound);
            }

            false
        }
    };

    // limit the limit
    let limit = match form.limit.as_ref() {
        Some(&limit) if limit < 100 => limit as i16,
        _ => 100,
    };

    // Deserializing the cursor should honestly never panic, so allow this branch to optimize better
    let cursor = match form.query.deserialize_simple().expect("Unable to deserialize cursor") {
        Some(query) => query,
        _ => Cursor::Before(MessageId::max_safe_value()),
    };

    // If the cursor is an exact message ID, use the single message request for efficiency
    // NOTE: This behavior must be documented in the API spec
    let req = match cursor {
        Cursor::Exact(msg_id) => GetMsgRequest::Single { msg_id },
        cursor => GetMsgRequest::Many {
            user_id: auth.user_id(),
            room_id,
            needs_perms,
            cursor,
            parent: form.parent.deserialize_simple().expect("Unable to deserialize parent"),
            limit,
            pins: form.pinned.as_slice(),
            starred: form.starred,
            recurse: form.recurse.min(5) as i16,
        },
    };

    let db = state.db.read.get().await?;

    crate::internal::get_messages::get_messages(state, &*db, req).await
}
