#[macro_use]
extern crate serde;

pub mod emote;
pub mod file;
pub mod gateway;
pub mod invite;
pub mod message;
pub mod party;
pub mod permission;
pub mod presence;
pub mod role;
pub mod room;
pub mod session;
pub mod sf;
pub mod user;

pub use self::{
    emote::*, file::*, gateway::*, invite::*, message::*, party::*, permission::*, presence::*, role::*,
    room::*, session::*, sf::*, user::*,
};

#[inline]
pub const fn is_false(value: &bool) -> bool {
    !*value
}
