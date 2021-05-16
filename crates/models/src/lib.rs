#[macro_use]
extern crate serde;

pub mod emote;
pub mod file;
pub mod invite;
pub mod message;
pub mod party;
pub mod permission;
pub mod role;
pub mod room;
pub mod session;
pub mod sf;
pub mod user;

pub use self::{
    emote::*, file::*, invite::*, message::*, party::*, permission::*, role::*, room::*,
    session::*, sf::*, user::*,
};

#[inline]
pub const fn is_false(value: &bool) -> bool {
    !*value
}
