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

#[derive(Deserialize)]
struct PatchPartyForm {
    name: String,

    #[serde(default)]
    owner_id: Option<Snowflake>,
}

pub async fn patch(mut route: Route, auth: Authorization, party_id: Snowflake) -> impl Reply {
    let form = match body::any::<PatchPartyForm>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    return "".into_response();
}
