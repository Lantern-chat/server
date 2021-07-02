use ftl::{
    ws::{WebSocketConfig, Ws, WsError},
    *,
};

use crate::web::{gateway::socket::client_connected, routes::api::ApiError};

pub fn gateway(route: Route<crate::ServerState>) -> Response {
    let addr = match real_ip::get_real_ip(&route) {
        Ok(addr) => addr,
        Err(_) => return ApiError::bad_request().into_response(),
    };

    let query = match route.query() {
        Some(Ok(query)) => query,
        None => Default::default(),
        _ => return ApiError::bad_request().into_response(),
    };

    // TODO: Move this into FTL websocket part?
    let state = route.state.clone();

    let mut config = WebSocketConfig::default();
    //config.max_message_size = Some(1024 * 512); // 512KIB
    config.max_send_queue = Some(1);

    match Ws::new(route, Some(config)) {
        Ok(ws) => ws
            .on_upgrade(move |ws| client_connected(ws, query, addr, state))
            .into_response(),
        Err(e) => e.into_response(),
    }
}
