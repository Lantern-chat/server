use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Auth token encoded as base-64
    pub auth: String,
    /// Expiration timestamp encoded with RFC 3339
    pub expires: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousSession {
    /// Expiration timestamp encoded with RFC 3339
    pub expires: String,
}
