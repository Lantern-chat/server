use sdk::models::{Nullable, Snowflake};

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetMode {
    Avatar,
    Banner,
}

pub async fn maybe_add_asset(
    state: &ServerState,
    mode: AssetMode,
    user_id: Snowflake,
    file_id: Nullable<Snowflake>,
) -> Result<Nullable<Snowflake>, Error> {
    Ok(Nullable::Null)
}
