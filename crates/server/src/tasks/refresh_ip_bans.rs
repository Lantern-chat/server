use std::time::Duration;

use crate::ServerState;

pub async fn refresh_ip_bans(state: ServerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 15));

    loop {
        let _now = tokio::select! {
            biased;
            now = interval.tick() => { now },
            _ = state.notify_shutdown.notified() => { break; }
        };

        log::trace!("Refreshing IP Bans");
        if let Err(e) = state.ip_bans.refresh(&state.db.read).await {
            log::error!("Error refreshing IP Bans! {e}");
        }
    }
}
