use sdk::{
    models::gateway::message::{ClientMsg, ServerMsg},
    models::sf::{NicheSnowflake, Snowflake},
};

use smallvec::{smallvec, SmallVec};

pub type SmallSnowflakeVec = SmallVec<[Snowflake; 1]>;

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub enum ServerEvent {
    Regular {
        msg: ServerMsg,

        #[with(NicheSnowflake)]
        room_id: Option<Snowflake>,

        user_ids: SmallSnowflakeVec,
        party_ids: SmallSnowflakeVec,
    },
    BulkUserBlockedRefresh {
        blocked: Vec<Snowflake>,
    },
    UserBlockedAdd {
        user_id: Snowflake,
    },
    UserBlockedRemove {
        user_id: Snowflake,
    },
}

impl ServerEvent {
    pub fn new(
        event: impl Into<ServerMsg>,
        user_ids: SmallSnowflakeVec,
        party_ids: SmallSnowflakeVec,
        room_id: Option<Snowflake>,
    ) -> Self {
        ServerEvent::Regular {
            msg: event.into(),
            room_id,
            user_ids,
            party_ids,
        }
    }

    pub fn party(event: impl Into<ServerMsg>, party_id: Snowflake, room_id: Option<Snowflake>) -> Self {
        ServerEvent::new(event, SmallVec::new(), smallvec![party_id], room_id)
    }

    pub fn user(event: impl Into<ServerMsg>, user_id: Snowflake, room_id: Option<Snowflake>) -> Self {
        ServerEvent::new(event, smallvec![user_id], SmallVec::new(), room_id)
    }
}
