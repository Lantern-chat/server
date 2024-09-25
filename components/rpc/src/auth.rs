use sdk::models::{ElevationLevel, Snowflake, Timestamp, UserFlags};

use auth::UserToken;

/// User and Bot authorization structure, optimized for branchless user_id lookup
///
/// These are typically cached in the gateway for faster reauth
#[derive(Debug, Clone, Copy, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[repr(u64, C)] // make sure things are tidy enough for branchless lookup of user_id/bot_id
pub enum Authorization {
    User {
        user_id: Snowflake,
        expires: Timestamp,
        token: UserToken,
        flags: UserFlags,
    },
    Bot {
        bot_id: Snowflake,
        issued: Timestamp,
    },
}

impl Authorization {
    #[inline(always)]
    pub const fn is_bot(&self) -> bool {
        matches!(self, Authorization::Bot { .. })
    }

    #[inline(always)]
    pub const fn is_user(&self) -> bool {
        matches!(self, Authorization::User { .. })
    }

    pub const fn is_admin(&self) -> bool {
        matches!(self, Authorization::User { flags, .. } if matches!(flags.elevation(), ElevationLevel::Staff | ElevationLevel::System))
    }

    #[inline(always)]
    pub const fn user_id(&self) -> Snowflake {
        *self.user_id_ref()
    }

    #[inline(always)]
    pub const fn user_id_ref(&self) -> &Snowflake {
        match self {
            Authorization::User { user_id, .. } => user_id,
            Authorization::Bot { bot_id, .. } => bot_id,
        }
    }
}
