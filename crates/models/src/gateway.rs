use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyEvent {
    pub user: User,
    pub dms: Vec<Room>,
    pub parties: Vec<Party>,
    pub session: Snowflake,
}
