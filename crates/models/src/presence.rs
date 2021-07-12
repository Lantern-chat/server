bitflags::bitflags! {
    pub struct UserPresenceFlags: i16 {
        const ONLINE    = 1 << 0;
        const AWAY      = 1 << 1;
        const BUSY      = 1 << 2;
        const MOBILE    = 1 << 3;
    }
}

serde_shims::impl_serde_for_bitflags!(UserPresenceFlags);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    pub flags: UserPresenceFlags,

    /// Updated-At timestamp as ISO-8061
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<Activity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {}
