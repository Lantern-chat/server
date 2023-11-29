use super::*;

use sdk::api::commands::party::CreatePartyInviteBody;

#[async_recursion]
pub async fn post(mut route: Route<ServerState>, auth: Authorization) -> WebResult {
    let form = body::any::<CreatePartyInviteBody, _>(&mut route).await?;

    Ok(().into())
}
