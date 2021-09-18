use super::*;

bitflags::bitflags! {
    pub struct Intent: u32 {
        /// - PARTY_CREATE
        /// - PARTY_UPDATE
        /// - PARTY_DELETE
        /// - PARTY_ROLE_CREATE
        /// - PARTY_ROLE_UPDATE
        /// - PARTY_ROLE_DELETE
        /// - CHANNEL_CREATE
        /// - CHANNEL_UPDATE
        /// - CHANNEL_DELETE
        /// - CHANNEL_PINS_UPDATE
        const PARTIES                   = 1 << 0;

        /// - PARTY_MEMBER_ADD
        /// - PARTY_MEMBER_UPDATE
        /// - PARTY_MEMBER_REMOVE
        const PARTY_MEMBERS             = 1 << 1;

        /// - PARTY_BAN_ADD
        /// - PARTY_BAN_REMOVE
        const PARTY_BANS                = 1 << 2;

        /// - PARTY_EMOJIS_UPDATE
        const PARTY_EMOTES              = 1 << 3;

        /// - PARTY_INTEGRATIONS_UPDATE
        /// - INTEGRATION_CREATE
        /// - INTEGRATION_UPDATE
        /// - INTEGRATION_DELETE
        const PARTY_INTEGRATIONS        = 1 << 4;

        /// - WEBHOOKS_UPDATE
        const PARTY_WEBHOOKS            = 1 << 5;

        /// - INVITE_CREATE
        /// - INVITE_DELETE
        const PARTY_INVITES             = 1 << 6;

        /// - VOICE_STATE_UPDATE
        const VOICE_STATUS              = 1 << 7;

        /// - PRESENCE_UPDATE
        const PRESENCE                  = 1 << 8;

        /// - MESSAGE_CREATE
        /// - MESSAGE_UPDATE
        /// - MESSAGE_DELETE
        /// - MESSAGE_DELETE_BULK
        const MESSAGES                  = 1 << 9;

        /// - MESSAGE_REACTION_ADD
        /// - MESSAGE_REACTION_REMOVE
        /// - MESSAGE_REACTION_REMOVE_ALL
        /// - MESSAGE_REACTION_REMOVE_EMOTE
        const MESSAGE_REACTIONS         = 1 << 10;

        /// - TYPING_START
        const MESSAGE_TYPING            = 1 << 11;

        /// - MESSAGE_CREATE
        /// - MESSAGE_UPDATE
        /// - MESSAGE_DELETE
        /// - CHANNEL_PINS_UPDATE
        const DIRECT_MESSAGES           = 1 << 12;

        /// - MESSAGE_REACTION_ADD
        /// - MESSAGE_REACTION_REMOVE
        /// - MESSAGE_REACTION_REMOVE_ALL
        /// - MESSAGE_REACTION_REMOVE_EMOTE
        const DIRECT_MESSAGE_REACTIONS  = 1 << 13;

        /// - TYPING_START
        const DIRECT_MESSAGE_TYPING     = 1 << 14;
    }
}

serde_shims::bitflags::impl_serde_for_bitflags!(Intent);

pub mod commands {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Identify {
        pub auth: SmolStr,
        pub intent: Intent,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SetPresence {
        #[serde(flatten)]
        pub presence: UserPresence,
    }
}

pub mod events {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Hello {
        /// Number of milliseconds between heartbeats
        pub heartbeat_interval: u32,
    }

    impl Default for Hello {
        fn default() -> Self {
            Hello {
                heartbeat_interval: 45000, // 45 seconds
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Ready {
        pub user: User,
        pub dms: Vec<Room>,
        pub parties: Vec<Party>,
        pub session: Snowflake,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TypingStart {
        pub room: Snowflake,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub party: Option<Snowflake>,
        pub user: Snowflake,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub member: Option<PartyMember>,
        // maybe timestamp?
        //ts: u32,
    }

    //#[derive(Debug, Clone, Serialize, Deserialize)]
    //pub struct PresenceUpdate {
    //    pub user_id: Snowflake,
    //    pub presence: UserPresence,
    //}
}
