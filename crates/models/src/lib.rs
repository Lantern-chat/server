#![allow(unused_imports)]

#[macro_use]
extern crate serde;

use smol_str::SmolStr;

pub mod emote;
pub mod file;
pub mod gateway;
pub mod invite;
pub mod message;
pub mod party;
pub mod permission;
pub mod prefs;
pub mod presence;
pub mod role;
pub mod room;
pub mod session;
pub mod sf;
pub mod user;

pub use self::{
    emote::*, file::*, gateway::*, invite::*, message::*, party::*, permission::*, prefs::*, presence::*,
    role::*, room::*, session::*, sf::*, user::*,
};

#[allow(unused)]
#[inline]
pub(crate) const fn is_false(value: &bool) -> bool {
    !*value
}

#[allow(unused)]
#[inline]
pub(crate) const fn is_true(value: &bool) -> bool {
    *value
}

#[allow(unused)]
#[inline]
pub(crate) fn is_none_or_empty<T>(value: &Option<Vec<T>>) -> bool {
    match value {
        None => true,
        Some(v) => v.is_empty(),
    }
}

#[allow(unused)]
#[inline]
pub(crate) fn default_true() -> bool {
    true
}

#[allow(unused)]
#[inline]
pub(crate) fn is_default<T>(value: &T) -> bool
where
    T: Default + PartialEq,
{
    *value == T::default()
}
