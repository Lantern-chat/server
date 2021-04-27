use super::{Event, EventError, GatewayConnection, ServerState};
use crate::gateway::msg::{server::*, ServerMsg};

use crate::routes::api::auth::{do_auth, Authorization};

pub async fn identify(state: ServerState, conn: GatewayConnection, auth: String, intent: u32) {
    let auth = match do_auth(&state, auth.as_bytes()).await {
        Ok(auth) => auth,
        Err(_) => {
            conn.tx.send(super::INVALID_SESSION.clone()).await;
            return;
        }
    };
}
