use std::{sync::Arc, time::SystemTime};

use warp::{
    body::json,
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
struct PartyCreateForm {
    name: String,
}

pub fn create(
    state: ServerState,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::post()
        .and(warp::path::end())
        .and(auth(state.clone()))
        .and(warp::body::form::<PartyCreateForm>())
        .and(state.inject())
        .and_then(|auth, form, state| async move {
            match create_party(state, auth, form).await {
                Ok(ref new_party) => Ok::<_, Rejection>(warp::reply::with_status(
                    warp::reply::json(new_party),
                    StatusCode::OK,
                )),
                Err(e) => match e {
                    CreatePartyError::ClientError(_) => {
                        log::error!("CreatePartyError: {}", e);
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
pub enum CreatePartyError {
    #[error("Name is too short or too long")]
    NameError,

    #[error("Database Error: {0}")]
    ClientError(#[from] ClientError),
}

#[derive(Serialize)]
pub struct NewParty {
    id: Snowflake,
    name: String,
}

async fn create_party(
    state: ServerState,
    auth: Authorization,
    form: PartyCreateForm,
) -> Result<NewParty, CreatePartyError> {
    let id = Snowflake::now();

    if form.name.len() < 4 || form.name.len() > 256 {
        return Err(CreatePartyError::NameError);
    }

    // TODO: Decide if a user should have a maximum number of parties

    state
        .db
        .execute_cached(
            || "INSERT INTO lantern.rooms (id, name, owner_id) VALUES ($1, $2, $3)",
            &[&id, &form.name, &auth.user_id],
        )
        .await?;

    Ok(NewParty {
        id,
        name: form.name,
    })
}
