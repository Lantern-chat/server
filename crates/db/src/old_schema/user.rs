use super::*;

use time::PrimitiveDateTime;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    pub discriminator: i16,

    pub email: Option<String>,
    pub nickname: Option<String>,
    pub blurb: Option<String>,
    pub avatar_id: Option<Snowflake>,
    pub preferences: Option<String>, // JSON
}

impl User {}
