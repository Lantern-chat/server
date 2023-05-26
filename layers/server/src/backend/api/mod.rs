#![allow(clippy::redundant_closure)]

pub mod admin;
pub mod auth;
pub mod perm;

#[derive(Debug, Clone, Copy)]
pub enum SearchMode<'a> {
    Single(schema::Snowflake),
    Many(&'a [schema::Snowflake]),
}

pub mod gateway {
    pub mod presence;
    pub mod ready;
    pub mod refresh;
}

pub mod user {
    pub mod get;
    pub mod register;

    pub mod me {
        pub mod account;
        pub mod get;
        pub mod login;
        pub mod logout;
        pub mod prefs;
        pub mod profile;
        pub mod sessions;

        pub mod relationships {
            pub mod get;
            pub mod modify;
        }
    }
}

pub mod party {
    pub mod create;
    pub mod emotes;
    pub mod get;
    pub mod members;
    pub mod modify;
    pub mod roles;
    pub mod stats;

    pub mod rooms {
        pub mod create;
        pub mod get;
    }
}

pub mod room {
    pub mod get;
    pub mod modify;
    pub mod remove;
    pub mod typing;

    pub mod messages {
        pub mod create;
        pub mod delete;
        pub mod edit;
        pub mod get;

        pub mod reaction {
            pub mod add;
            pub mod remove;
        }
    }

    pub mod threads {
        pub mod edit;
        pub mod get;
    }
}

pub mod invite {
    pub mod create;
    pub mod get;
    pub mod redeem;
    pub mod revoke;
}

pub mod file {
    pub mod delete;
    pub mod head;
    pub mod options;
    pub mod patch;
    pub mod post;
}

pub mod metrics;

pub mod oembed {
    pub mod get;
}
