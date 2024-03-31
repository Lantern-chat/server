use sdk::Snowflake;

use crate::procedure::Procedure;

#[derive(Debug, rkyv::Archive, rkyv::Serialize)]
#[archive(check_bytes)]
pub enum RpcRequest {
    Procedure {
        proc: Procedure,
        addr: std::net::SocketAddr,
        auth: Option<crate::auth::Authorization>,
    },
    /// Fetch party info from a party_id
    GetPartyInfoFromPartyId(Snowflake),
    /// Fetch party info from a room_id
    GetPartyInfoFromRoomId(Snowflake),
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize)]
#[archive(check_bytes)]
pub struct PartyInfo {
    pub party_id: Snowflake,

    #[with(rkyv::with::CopyOptimize)]
    pub room_ids: Vec<Snowflake>,
}
