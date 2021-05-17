pub mod auth;
pub mod error;

pub use error::Error;

pub mod user {
    pub mod login;
    pub mod logout;
    pub mod register;
}
