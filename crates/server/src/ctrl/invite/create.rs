use std::str::FromStr;

use crate::{
    ctrl::Error,
    web::{auth::Authorization, routes::api::v1::file::post::Metadata},
    ServerState,
};

use db::pool::Object;
use schema::{flags::FileFlags, Snowflake, SnowflakeExt};

use rand::Rng;
use smol_str::SmolStr;
use timestamp::Timestamp;

#[derive(Debug, Deserialize)]
pub struct InviteOptions {
    pub party_id: Snowflake,
    pub expires: Option<Timestamp>,
}

pub async fn create_invite(state: ServerState, auth: Authorization) -> Result<(), Error> {
    unimplemented!()
}
