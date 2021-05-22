pub mod auth;
pub mod error;

pub use error::Error;

pub mod gateway {
    pub mod ready;
}

pub mod user {
    pub mod login;
    pub mod logout;
    pub mod register;
    pub mod sessions;
}

pub mod party {
    pub mod emotes;
    pub mod get;
    pub mod members;
    pub mod roles;
}

#[derive(Debug, Clone, Copy)]
pub enum SearchMode<'a> {
    Single(db::Snowflake),
    Many(&'a [db::Snowflake]),
}
