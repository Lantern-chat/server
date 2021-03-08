use crate::server::ftl::{
    ws::{Ws, WsError},
    *,
};

use crate::server::gateway::client_connected;

pub fn gateway(route: Route) -> Result<impl Reply, WsError> {
    let addr = match real_ip::get_real_ip(&route) {
        Ok(addr) => addr,
        Err(e) => return Ok(StatusCode::BAD_REQUEST.into_response()),
    };

    let query = match route.query() {
        Some(Ok(query)) => query,
        None => Default::default(),
        _ => return Ok(StatusCode::BAD_REQUEST.into_response()),
    };

    let state = route.state.clone();

    Ok(Ws::new(route)?
        .on_upgrade(move |ws| client_connected(ws, query, addr, state))
        .into_response())
}
