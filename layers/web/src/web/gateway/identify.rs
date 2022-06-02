use sdk::models::{AuthToken, BearerToken, Intent};

use super::{Event, EventError, GatewayConnection, ServerState};

use crate::ctrl::{
    auth::{do_auth, Authorization},
    gateway::ready::ready,
    Error,
};

use sdk::models::gateway::message::ServerMsg;

pub async fn identify(state: ServerState, conn: GatewayConnection, auth: AuthToken, intent: Intent) {
    if let Err(e) = do_identify(state, &conn, auth, intent).await {
        log::error!("Error identifying and sending ready event: {e}");
        let _ = conn.tx.send(super::INVALID_SESSION.clone()).await;
    }
}

async fn do_identify(state: ServerState, conn: &GatewayConnection, auth: AuthToken, _intent: Intent) -> Result<(), Error> {
    let auth = do_auth(&state, auth.try_into()?).await?;
    let ready = ready(state, conn.id, auth).await?;
    let _ = conn.tx.send(Event::new(ServerMsg::new_ready(Box::new(ready)), None)?).await;
    Ok(())
}
