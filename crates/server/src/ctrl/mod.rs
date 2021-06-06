pub mod auth;
pub mod error;
pub mod perm;

pub use error::Error;

#[derive(Debug, Clone, Copy)]
pub enum SearchMode<'a> {
    Single(db::Snowflake),
    Many(&'a [db::Snowflake]),
}

pub mod gateway {
    pub mod ready;
}

pub mod user {
    pub mod register;

    pub mod me {
        pub mod friends;
        pub mod login;
        pub mod logout;
        pub mod sessions;
    }
}

pub mod party {
    pub mod create;
    pub mod emotes;
    pub mod get;
    pub mod members;
    pub mod roles;

    pub mod rooms {
        pub mod get;
    }
}

pub mod room {
    pub mod get;

    pub mod messages {
        pub mod create;
        pub mod get_many;
        pub mod get_one;
    }
}
