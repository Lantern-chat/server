use futures::{
    stream::{self, AbortHandle, BoxStream, SelectAll},
    StreamExt,
};

use tokio_stream::wrappers::BroadcastStream;

use hashbrown::HashMap;

use crate::prelude::*;

#[repr(transparent)]
#[derive(Debug, Default)]
pub struct ListenerTable {
    table: HashMap<PartyId, AbortHandle, sdk::FxRandomState2>,
}

impl std::ops::Deref for ListenerTable {
    type Target = HashMap<PartyId, AbortHandle, sdk::FxRandomState2>;

    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

use crate::gateway::{handler::Item, Subscriptions};

impl ListenerTable {
    // TODO: to implement sub/unsub low-bandwidth modes, the abort handles will
    // probably need to be replaced with something that pauses them instead
    pub fn register_subs(&mut self, events: &mut SelectAll<BoxStream<Item>>, subs: Subscriptions) {
        let subs = subs.parties.into_iter().chain(subs.rooms);

        // iterate through subscribed parties
        events.extend(subs.map(|sub| {
            // take their broadcast stream and wrap it in an abort signal
            // this is so we can unsubscribe later if needed
            let (stream, handle) = stream::abortable(BroadcastStream::new(sub.rx));

            self.table.insert(sub.id, handle);

            // map the stream to the `Item` type
            stream.map(|event| Item::Event(event.map_err(Into::into))).boxed()
        }));
    }
}
