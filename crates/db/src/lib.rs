#![allow(unused_imports)]

#[macro_use]
extern crate serde;

extern crate tracing as log;

pub mod client;
pub mod conn;
pub mod startup;
pub mod util;

pub use client::{ClientError, ReadWriteClient as Client};

pub mod sf;
pub use sf::{Snowflake, SnowflakeExt};

pub mod schema {
    pub(self) use super::{Client, ClientError, Snowflake, SnowflakeExt};

    pub mod attachment;
    pub mod emote;
    pub mod file;
    pub mod invite;
    pub mod msg;
    pub mod party;
    pub mod role;
    pub mod room;
    pub mod thread;
    pub mod user;

    pub use self::{
        attachment::Attachment,
        emote::Emote,
        invite::Invite,
        msg::{Message, MessageSearch},
        party::Party,
        role::Role,
        room::{Room, RoomFlags},
        thread::Thread,
        user::User,
    };
}
