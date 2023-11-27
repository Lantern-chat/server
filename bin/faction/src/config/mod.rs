pub mod sections {
    pub mod account;
    pub mod db;
    pub mod general;
    pub mod keys;
    pub mod message;
    pub mod party;
    pub mod paths;
    pub mod services;
    pub mod upload;
    pub mod user;
    pub mod web;
}

config::config! {
    pub struct Config {
        /// Overall server configuration
        general: sections::general::General,
        /// Filesystem paths
        paths: sections::paths::Paths,
        /// Database configuration
        db: sections::db::Database,
        /// User account configuration
        account: sections::account::Account,
        /// Settings for individual messages
        message: sections::message::Message,
        /// Settings for parties
        party: sections::party::Party,
        /// User uploads configuration
        upload: sections::upload::Upload,
        /// Settings for services used by the backend
        services: sections::services::Services,
        /// Cryptographic keys
        keys: sections::keys::Keys,
        /// Web/HTTP Configuration
        web: sections::web::Web,
        /// User-related Configuration
        user: sections::user::User,
    }
}
