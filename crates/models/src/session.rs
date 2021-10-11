use super::*;

pub type SmolToken = arrayvec::ArrayString<28>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Auth token encoded as base-64
    pub auth: SmolToken,
    /// Expiration timestamp encoded with RFC 3339
    pub expires: SmolStr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousSession {
    /// Expiration timestamp encoded with RFC 3339/ISO 8061
    pub expires: SmolStr,
}
