use futures::{
    stream::{self, AbortHandle, BoxStream, SelectAll},
    StreamExt,
};

use tokio_stream::wrappers::BroadcastStream;

use hashbrown::HashMap;
use sdk::Snowflake;

#[repr(transparent)]
#[derive(Debug, Default)]
pub struct ListenerTable {
    table: HashMap<Snowflake, AbortHandle>,
}

impl std::ops::Deref for ListenerTable {
    type Target = HashMap<Snowflake, AbortHandle>;

    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

use crate::gateway::{handler::Item, PartySubscription};

impl ListenerTable {
    // TODO: to implement sub/unsub low-bandwidth modes, the abort handles will
    // probably need to be replaced with something that pauses them instead
    pub fn register_subs(&mut self, events: &mut SelectAll<BoxStream<Item>>, subs: Vec<PartySubscription>) {
        // iterate through subscribed parties
        events.extend(subs.into_iter().map(|sub| {
            // take their broadcast stream and wrap it in an abort signal
            // this is so we can unsubscribe later if needed
            let (stream, handle) = stream::abortable(BroadcastStream::new(sub.rx));

            self.table.insert(sub.party_id, handle);

            // map the stream to the `Item` type
            stream.map(|event| Item::Event(event.map_err(Into::into))).boxed()
        }));
    }
}
