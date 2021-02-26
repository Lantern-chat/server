use rand::Rng;
use std::{sync::Arc, time::SystemTime};

use http::StatusCode;

use crate::{
    db::{schema::Party, ClientError, Snowflake},
    server::{
        routes::api::{auth::Authorization, util::time::is_of_age},
        ServerState,
    },
};

use crate::server::ftl::*;

#[derive(Debug, Clone, Deserialize)]
struct PartyCreateForm {
    name: String,
}

pub async fn post(mut route: Route, auth: Authorization) -> impl Reply {
    let form = match body::any::<PartyCreateForm>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    match create_party(route.state, form, auth).await {
        Ok(ref party) => reply::json(party).into_response(),
        Err(err) => "".into_response(),
    }
}

#[derive(Debug, thiserror::Error)]
enum PartyCreateError {
    #[error("Invalid Party Name")]
    InvalidName,

    #[error("Database Error {0}")]
    ClientError(#[from] ClientError),
}

async fn create_party(
    state: ServerState,
    form: PartyCreateForm,
    auth: Authorization,
) -> Result<Party, PartyCreateError> {
    if !state.config.partyname_len.contains(&form.name.len()) {
        return Err(PartyCreateError::InvalidName);
    }

    let party = Party {
        id: Snowflake::now(),
        owner_id: auth.user_id,
        name: form.name,
    };

    party.insert(&state.db).await?;

    Ok(party)
}
