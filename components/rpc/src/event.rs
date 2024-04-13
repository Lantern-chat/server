use sdk::{
    models::gateway::message::{ClientMsg, ServerMsg},
    models::sf::{NicheSnowflake, Snowflake},
};

use smallvec::{smallvec, SmallVec};

pub type SmallSnowflakeVec = SmallVec<[Snowflake; 1]>;

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub enum ClientCommand {
    /// Regular client message/command
    Regular(ClientMsg),
}

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
        user_ids: SmallSnowflakeVec,
        party_ids: SmallSnowflakeVec,
        room_id: Option<Snowflake>,
        event: impl Into<ServerMsg>,
    ) -> Self {
        ServerEvent::Regular {
            msg: event.into(),
            room_id,
            user_ids,
            party_ids,
        }
    }

    pub fn new_iter(
        user_ids: impl IntoIterator<Item = Snowflake>,
        party_ids: impl IntoIterator<Item = Snowflake>,
        room_id: Option<Snowflake>,
        event: impl Into<ServerMsg>,
    ) -> Self {
        ServerEvent::new(
            SmallSnowflakeVec::from_iter(user_ids),
            SmallSnowflakeVec::from_iter(party_ids),
            room_id,
            event,
        )
    }

    pub fn party(party_id: Snowflake, room_id: Option<Snowflake>, event: impl Into<ServerMsg>) -> Self {
        ServerEvent::new(SmallVec::new(), smallvec![party_id], room_id, event)
    }

    pub fn user(user_id: Snowflake, room_id: Option<Snowflake>, event: impl Into<ServerMsg>) -> Self {
        ServerEvent::new(smallvec![user_id], SmallVec::new(), room_id, event)
    }
}
