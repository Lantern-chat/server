use ftl::*;

use db::{
    schema::{Message, Room},
    ClientError, Snowflake, SnowflakeExt,
};

use crate::{routes::api::auth::Authorization, ServerState};

#[derive(Debug, Deserialize)]
pub struct MessagePostForm {
    content: String,
}

pub async fn post(
    mut route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> impl Reply {
    let form = match body::any::<MessagePostForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match post_message(route.state, auth, room_id, form).await {
        Ok(ref msg) => reply::json(msg).into_response(),
        Err(e) => match e {
            MessagePostError::ClientError(e) => {
                log::error!("Error posting message: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            _ => e
                .to_string()
                .with_status(StatusCode::BAD_REQUEST)
                .into_response(),
        },
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MessagePostError {
    #[error("Invalid Message Content")]
    InvalidMessageContent,

    #[error(transparent)]
    ClientError(#[from] ClientError),
}

pub async fn post_message(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    form: MessagePostForm,
) -> Result<Message, MessagePostError> {
    if !state.config.message_len.contains(&form.content.len()) {
        return Err(MessagePostError::InvalidMessageContent);
    }

    let newlines = form.content.chars().filter(|c| *c == '\n').count();

    if state.config.max_message_newlines < newlines {
        return Err(MessagePostError::InvalidMessageContent);
    }

    let message = Message {
        id: Snowflake::now(),
        user_id: auth.user_id,
        room_id,
        editor_id: None,
        thread_id: None,
        updated_at: None,
        deleted_at: None,
        content: form.content,
        pinned: false,
    };

    message.upsert(&state.db).await?;

    Ok(message)
}
