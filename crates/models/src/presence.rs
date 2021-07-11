#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde_repr::Serialize_repr,
    serde_repr::Deserialize_repr,
)]
#[repr(i16)]
pub enum UserStatus {
    Online = 0,
    Away = 1,
    Busy = 2,
    Offline = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    pub status: UserStatus,
}
