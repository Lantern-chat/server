use ftl::*;

use db::{schema::Party, ClientError, Snowflake};

use crate::{
    routes::api::{auth::Authorization, util::time::is_of_age},
    ServerState,
};

#[derive(Deserialize)]
struct PatchPartyForm {
    name: String,

    #[serde(default)]
    owner_id: Option<Snowflake>,
}

pub async fn patch(
    mut route: Route<ServerState>,
    auth: Authorization,
    party_id: Snowflake,
) -> Response {
    let form = match body::any::<PatchPartyForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return e.into_response(),
    };

    return "".into_response();
}
