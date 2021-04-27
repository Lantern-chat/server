use std::{sync::Arc, time::SystemTime};

use warp::{
    body::json_or_form,
    hyper::{Server, StatusCode},
    reject::Reject,
    Filter, Rejection, Reply,
};

use crate::{
    db::{ClientError, Snowflake},
    server::{
        auth::AuthToken,
        rate::RateLimitKey,
        routes::{
            api::ApiError,
            filters::{auth, Authorization},
        },
        ServerState,
    },
};

#[derive(Debug, Clone, Deserialize)]
struct RoomCreateForm {
    name: String,
    //topic: Option<String>,
    //#[serde(default)]
    //nsfw: bool,
}

pub fn create(
    state: ServerState,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::post()
        .and(warp::path::end())
        .and(auth(state.clone()))
        .and(warp::body::json_or_form::<RoomCreateForm>())
        .map(move |auth, form| (auth, form, state.clone()))
        .and_then(|(auth, form, state)| async move {
            match create_room(state, auth, form).await {
                Ok(ref new_room) => Ok::<_, Rejection>(warp::reply::with_status(
                    warp::reply::json(new_room),
                    StatusCode::OK,
                )),
                Err(e) => match e {
                    CreateRoomError::ClientError(_) => {
                        log::error!("CreateRoomError: {}", e);
                        Ok(warp::reply::with_status(
                            warp::reply::json(&ApiError {
                                code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                                message: "Internal Error".to_owned(),
                            }),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        ))
                    }
                    _ => Ok(warp::reply::with_status(
                        warp::reply::json(&ApiError {
                            code: StatusCode::BAD_REQUEST.as_u16(),
                            message: e.to_string(),
                        }),
                        StatusCode::BAD_REQUEST,
                    )),
                },
            }
        })
}

#[derive(Debug, thiserror::Error)]
pub enum CreateRoomError {
    #[error("Name is too short or too long")]
    NameError,

    #[error("Database Error: {0}")]
    ClientError(#[from] ClientError),
}

#[derive(Serialize)]
pub struct NewRoom {
    id: Snowflake,
    name: String,
}

async fn create_room(
    state: ServerState,
    auth: Authorization,
    form: RoomCreateForm,
) -> Result<NewRoom, CreateRoomError> {
    let id = Snowflake::now();

    if form.name.len() < 4 || form.name.len() > 256 {
        return Err(CreateRoomError::NameError);
    }

    // TODO: Decide if a user should have a maximum number of parties

    state
        .db
        .execute_cached(
            || "INSERT INTO lantern.rooms (id, name, owner_id) VALUES ($1, $2, $3)",
            &[&id, &form.name, &auth.user_id],
        )
        .await?;

    Ok(NewRoom {
        id,
        name: form.name,
    })
}
