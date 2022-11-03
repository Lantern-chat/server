use schema::SnowflakeExt;
use sdk::models::Snowflake;
use smol_str::SmolStr;

//use util::hex::HexidecimalInt;

use crate::ServerState;

/// Encrypt a snowflake and encode is as a hexidecimal string
pub fn encrypt_snowflake(state: &ServerState, id: Snowflake) -> SmolStr {
    util::base64::encode_u128(id.encrypt(state.config.keys.sf_key))
}

pub fn decrypt_snowflake(state: &ServerState, s: &str) -> Option<Snowflake> {
    let Ok(block) = util::base64::decode_u128(s) else { return None };

    Snowflake::decrypt(block, state.config.keys.sf_key)
}

#[inline]
pub fn encrypt_snowflake_opt<R>(state: &ServerState, id: Option<Snowflake>) -> Option<R>
where
    R: From<SmolStr>,
{
    id.map(|id| R::from(encrypt_snowflake(state, id)))
}
