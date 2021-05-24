use super::{Event, EventError, GatewayConnection, ServerState};

use crate::ctrl::{
    auth::{do_auth, Authorization},
    gateway::ready::ready,
    Error,
};
use crate::web::gateway::msg::{server::*, ServerMsg};

pub async fn identify(state: ServerState, conn: GatewayConnection, auth: String, intent: u32) {
    if let Err(e) = do_identify(state, &conn, auth, intent).await {
        log::error!("Error identifying and sending ready event: {}", e);
        let _ = conn.tx.send(super::INVALID_SESSION.clone()).await;
    }
}

async fn do_identify(
    state: ServerState,
    conn: &GatewayConnection,
    auth: String,
    _intent: u32,
) -> Result<(), Error> {
    let auth = do_auth(&state, auth.as_bytes()).await?;
    let ready = ready(state, conn.id, auth).await?;
    let _ = conn.tx.send(Event::new_ready(ready)?).await;
    Ok(())
}