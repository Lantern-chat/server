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
