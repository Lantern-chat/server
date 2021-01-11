pub mod client;
pub mod conn;
pub mod startup;

pub use client::{Client, ClientError};

pub mod sf;
pub use sf::Snowflake;

pub mod schema {
    use super::{Client, ClientError, Snowflake};
    use tokio_postgres::{Error, Row};

    pub mod attachment;
    pub mod emote;
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
        msg::Message,
        party::Party,
        role::Role,
        room::{Room, RoomKind},
        thread::Thread,
        user::User,
    };
}
