use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloEvent {
    /// Number of milliseconds between heartbeats
    pub heartbeat_interval: u32,
}

impl Default for HelloEvent {
    fn default() -> Self {
        HelloEvent {
            heartbeat_interval: 45000, // 45 seconds
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyEvent {
    pub user: User,
    pub dms: Vec<Room>,
    pub parties: Vec<Party>,
    pub session: Snowflake,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingStartEvent {
    pub room: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party: Option<Snowflake>,
    pub user: Snowflake,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<PartyMember>,
    // maybe timestamp?
    //ts: u32,
}
