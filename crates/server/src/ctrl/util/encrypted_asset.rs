use models::Snowflake;
use schema::SnowflakeExt;

use crate::{util::hex::HexidecimalInt, ServerState};

/// Encrypt a snowflake and encode is as a hexidecimal string
pub fn encrypt_snowflake(state: &ServerState, id: Snowflake) -> String {
    HexidecimalInt(id.encrypt(state.config.sf_key)).to_string()
}

#[inline]
pub fn encrypt_snowflake_opt(state: &ServerState, id: Option<Snowflake>) -> Option<String> {
    id.map(|id| encrypt_snowflake(state, id))
}
