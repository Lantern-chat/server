use models::Snowflake;
use schema::SnowflakeExt;

use crate::{util::hex::HexidecimalInt, ServerState};

/// Encrypt a snowflake and encode is as a hexidecimal string
pub fn encrypt_snowflake(state: &ServerState, id: Snowflake) -> String {
    HexidecimalInt(id.encrypt(state.config.sf_key)).to_string()
}
