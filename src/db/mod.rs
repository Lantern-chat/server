pub mod client;
pub mod conn;
pub mod queries;

pub use client::Client;

pub mod sf;
pub use sf::Snowflake;

pub mod schema {
    use super::{queries::CachedQuery, Client, Snowflake};
    use tokio_postgres::{Error, Row};

    pub mod emote;
    pub mod invite;
    pub mod msg;
    pub mod party;
    pub mod role;
    pub mod room;
    pub mod user;

    pub use self::{
        emote::Emote,
        invite::Invite,
        msg::Message,
        party::Party,
        role::Role,
        room::{Room, RoomKind},
        user::User,
    };
}
