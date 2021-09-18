use models::Snowflake;
use schema::SnowflakeExt;
use smol_str::SmolStr;

use util::hex::HexidecimalInt;

use crate::ServerState;

/// Encrypt a snowflake and encode is as a hexidecimal string
pub fn encrypt_snowflake(state: &ServerState, id: Snowflake) -> SmolStr {
    HexidecimalInt(id.encrypt(state.config.sf_key)).to_hex()
}

#[inline]
pub fn encrypt_snowflake_opt<R>(state: &ServerState, id: Option<Snowflake>) -> Option<R>
where
    R: for<'a> From<&'a str>,
{
    id.map(|id| R::from(encrypt_snowflake(state, id).as_str()))
}
