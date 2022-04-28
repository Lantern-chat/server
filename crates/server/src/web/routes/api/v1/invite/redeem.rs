use ftl::*;
use sdk::api::commands::invite::RedeemInviteBody;
use smol_str::SmolStr;

use crate::{
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};

pub async fn redeem(mut route: Route<ServerState>, auth: Authorization, code: SmolStr) -> Response {
    let body = match body::any::<RedeemInviteBody, _>(&mut route).await {
        Ok(body) => body,
        Err(e) => return e.into_response(),
    };

    match crate::ctrl::invite::redeem::redeem_invite(route.state, auth, code, body).await {
        Ok(_) => ().into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
