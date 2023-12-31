use sdk::{
    models::gateway::message::{ClientMsg, ServerMsg},
    models::sf::{NicheSnowflake, Snowflake},
};

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub struct ServerEvent {
    #[with(NicheSnowflake)]
    pub party_id: Option<Snowflake>,

    #[with(NicheSnowflake)]
    pub room_id: Option<Snowflake>,

    pub msg: ServerMsg,
}

impl ServerEvent {
    pub fn new(event: impl Into<ServerMsg>, party_id: Option<Snowflake>, room_id: Option<Snowflake>) -> Self {
        ServerEvent {
            party_id,
            room_id,
            msg: event.into(),
        }
    }
}
