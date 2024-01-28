use super::*;

use sdk::api::commands::party::CreatePartyInviteBody;

pub async fn post(mut route: Route<ServerState>, auth: Authorization) -> ApiResult {
    let form = body::any::<CreatePartyInviteBody, _>(&mut route).await?;

    unimplemented!()
}
