use ftl::{
    ws::{WebSocketConfig, Ws, WsError},
    *,
};

use crate::web::gateway::socket::client_connected;

pub fn gateway(route: Route<crate::ServerState>) -> Result<impl Reply, WsError> {
    let addr = match real_ip::get_real_ip(&route) {
        Ok(addr) => addr,
        Err(_) => return Ok(StatusCode::BAD_REQUEST.into_response()),
    };

    let query = match route.query() {
        Some(Ok(query)) => query,
        None => Default::default(),
        _ => return Ok(StatusCode::BAD_REQUEST.into_response()),
    };

    // TODO: Move this into FTL websocket part?
    let state = route.state.clone();

    let mut config = WebSocketConfig::default();
    //config.max_message_size = Some(1024 * 512); // 512KIB
    config.max_send_queue = Some(1);

    Ok(Ws::new(route, Some(config))?
        .on_upgrade(move |ws| client_connected(ws, query, addr, state))
        .into_response())
}
