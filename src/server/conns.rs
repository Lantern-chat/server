use std::{
    hash::{BuildHasher, Hash, Hasher},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use ahash::{AHasher, RandomState};
use futures::{
    stream::{iter, repeat},
    FutureExt, Stream, StreamExt,
};
use hashbrown::HashMap;
use tokio::sync::{Mutex, RwLock};

use crate::{db::Snowflake, server::gateway::conn::ClientConnection, util::cmap::CHashMap};

use super::events::Event;

#[derive(Default)]
pub struct HostConnections {
    pub conns: CHashMap<Snowflake, Arc<RwLock<Vec<ClientConnection>>>>,
}

impl HostConnections {
    pub async fn send_event(
        host: Arc<HostConnections>,
        users: impl Stream<Item = Snowflake>,
        event: Arc<Event>,
    ) -> usize {
        let count = Arc::new(AtomicUsize::new(0));

        // for the stream of users, send in host/event/count values
        users
            .zip(repeat((host, event, count.clone())))
            // spawn up to 32 tasks to process user events
            .for_each_concurrent(Some(32), |(user_id, (host, event, count))| {
                tokio::spawn(async move {
                    // if connected user by this ID
                    if let Some(conns) = host.conns.get_cloned(&user_id).await {
                        // read connections and iterate over each, sending the event
                        let conns = conns.read().await;
                        count.fetch_add(conns.len(), Ordering::Relaxed);
                        for conn in conns.iter() {
                            conn.process_event(event.clone()).await;
                        }
                    }
                })
                .map(|_| ())
            })
            .await;

        count.load(Ordering::Relaxed)
    }
}
