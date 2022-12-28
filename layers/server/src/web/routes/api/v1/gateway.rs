use super::*;

pub fn gateway(route: Route<crate::ServerState>) -> WebResult {
    let Ok(addr) = real_ip::get_real_ip(&route) else {
        return Err(Error::BadRequest);
    };

    let query = match route.query() {
        Some(Ok(query)) => query,
        None => Default::default(),
        _ => return Err(Error::BadRequest),
    };

    // TODO: Move this into FTL websocket part?
    let state = route.state.clone();

    let mut config = ws::WebSocketConfig::default();
    //config.max_message_size = Some(1024 * 512); // 512KIB
    config.max_send_queue = Some(1);

    let ws = ws::Ws::new(route, Some(config))?;

    Ok(ws
        .on_upgrade(move |ws| crate::web::gateway::client_connected(ws, query, addr, state))
        .into())
}
