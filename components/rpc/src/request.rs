use sdk::Snowflake;

use crate::{event::ClientCommand, procedure::Procedure};

#[derive(Debug, rkyv::Archive, rkyv::Serialize)]
pub enum RpcRequest {
    Procedure {
        proc: Procedure,
        addr: std::net::IpAddr,
        auth: Option<crate::auth::Authorization>,
    },
    /// Fetch party info from a party_id
    GetPartyInfoFromPartyId(Snowflake),
    /// Fetch party info from a room_id
    GetPartyInfoFromRoomId(Snowflake),

    ForwardedClientCommand(ClientCommand),
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize)]
pub struct PartyInfo {
    pub party_id: Snowflake,
    pub room_ids: Vec<Snowflake>,
}
