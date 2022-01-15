use ftl::*;

use db::{
    schema::{Party, Room, RoomFlags},
    ClientError, Snowflake, SnowflakeExt,
};

use crate::{routes::api::auth::Authorization, ServerState};

#[derive(Deserialize)]
pub struct RoomCreateForm {
    name: SmolStr,

    #[serde(default)]
    topic: Option<SmolStr>,

    #[serde(default)]
    parent_id: Option<Snowflake>,
}

pub async fn post_room(
    mut route: Route<ServerState>,
    auth: Authorization,
    party_id: Snowflake,
) -> Response {
    let form = match body::any::<RoomCreateForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return ApiError::bad_request().into_response(),
    };

    match create_room(route.state, form, auth, party_id).await {
        Ok(ref room) => reply::json(room).into_response(),
        Err(e) => match e {
            RoomCreateError::ClientError(e) => {
                log::error!("Room Create Error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            _ => ApiError::err(e).into_response(),
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
        position: 0,
        flags: RoomFlags::empty(),
        parent_id: None,
    };

    room.insert(&state.db).await?;

    Ok(room)
}
