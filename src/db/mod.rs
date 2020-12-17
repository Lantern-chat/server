use tokio_postgres::Client;

pub mod conn;

pub mod sf;

pub use sf::Snowflake;

pub mod schema {
    pub mod emote;
    pub mod invite;
    pub mod msg;
    pub mod party;
    pub mod role;
    pub mod room;
    pub mod user;
}
