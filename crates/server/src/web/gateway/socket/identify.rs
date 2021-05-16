use super::{Event, EventError, GatewayConnection, ServerState};

use crate::ctrl::auth::{do_auth, Authorization};
use crate::web::gateway::msg::{server::*, ServerMsg};

pub async fn identify(state: ServerState, conn: GatewayConnection, auth: String, _intent: u32) {
    let _auth = match do_auth(&state, auth.as_bytes()).await {
        Ok(auth) => auth,
        Err(_) => {
            let _ = conn.tx.send(super::INVALID_SESSION.clone()).await;
            return;
        }
    };
}
