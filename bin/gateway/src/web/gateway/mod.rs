pub mod event;
//pub mod identify;
pub mod conn;
pub mod heart;
pub mod internal;

pub use self::{event::Event, heart::Heart};

use sdk::models::gateway::message::ServerMsg;

lazy_static::lazy_static! {
    pub static ref HELLO_EVENT: Event = Event::new_compressed(ServerMsg::new_hello(sdk::models::events::Hello::default()), None, 10).unwrap();
    pub static ref HEARTBEAT_ACK: Event = Event::new_compressed(ServerMsg::new_heartbeat_ack(), None, 10).unwrap();
    pub static ref INVALID_SESSION: Event = Event::new_compressed(ServerMsg::new_invalid_session(), None, 10).unwrap();
}
