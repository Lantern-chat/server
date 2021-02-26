use crate::{
    db::{
        schema::{Party, Room, RoomFlags},
        ClientError, Snowflake,
    },
    server::{ftl::*, routes::api::auth::Authorization, ServerState},
};

#[derive(Deserialize)]
pub struct RoomCreateForm {
    name: String,

    #[serde(default)]
    topic: Option<String>,

    #[serde(default)]
    parent_id: Option<Snowflake>,
}

pub async fn post_room(mut route: Route, auth: Authorization, party_id: Snowflake) -> impl Reply {
    let form = match body::any::<RoomCreateForm>(&mut route).await {
        Ok(form) => form,
        Err(e) => return StatusCode::BAD_REQUEST.into_response(),
    };

    match create_room(route.state, form, auth, party_id).await {
        Ok(ref room) => reply::json(room).into_response(),
        Err(e) => match e {
            RoomCreateError::ClientError(e) => {
                log::error!("Room Create Error: {}", e);
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
enum RoomCreateError {
    #[error("Invalid Party Name")]
    InvalidName,

    #[error("Invalid topic value")]
    InvalidTopic,

    #[error("Database Error {0}")]
    ClientError(#[from] ClientError),
}

async fn create_room(
    state: ServerState,
    form: RoomCreateForm,
    auth: Authorization,
    party_id: Snowflake,
) -> Result<Room, RoomCreateError> {
    if !state.config.roomname_len.contains(&form.name.len()) {
        return Err(RoomCreateError::InvalidName);
    }

    match form.topic {
        Some(ref topic) if topic.len() > 2048 => {
            return Err(RoomCreateError::InvalidTopic);
        }
        _ => {}
    }

    let room = Room {
        id: Snowflake::now(),
        party_id,
        name: form.name,
        topic: form.topic,
        avatar_id: None,
        sort_order: 0,
        flags: RoomFlags::empty(),
        parent_id: None,
    };

    room.insert(&state.db).await?;

    Ok(room)
}
