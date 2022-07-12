#![allow(deprecated)]

thorn::enums! {
    /// **NOTE**: This must match `lantern.event_code` in the database **EXACTLY**, or it will fail.
    ///
    /// It will check for names and variant count.
    pub enum EventCode in Lantern {
        MessageCreate,
        MessageUpdate,
        MessageDelete,

        #[deprecated]
        TypingStarted,

        /// If any user updated with public fields
        UserUpdated,
        /// If self updated with private fields
        SelfUpdated,

        PresenceUpdated,

        PartyCreate,
        PartyUpdate,
        PartyDelete,

        RoomCreated,
        RoomUpdated,
        RoomDeleted,

        /// Per-party member information updated
        MemberUpdated,
        /// Member joined party
        MemberJoined,
        /// Member left party
        MemberLeft,
        /// Member was banned, only sent if proper gateway intent was used
        MemberBan,
        /// Member was unbanned, only sent if proper gateway intent was used
        MemberUnban,

        RoleCreated,
        RoleUpdated,
        RoleDeleted,

        InviteCreate,

        MessageReact,
        MessageUnreact,
    }
}

/// THIS MUST MATCH `lantern.to_language` in the database
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LanguageCode {
    English = 0,
    Arabic = 1,
    Armenian = 2,
    Basque = 3,
    Catalan = 4,
    Danish = 5,
    Dutch = 6,
    Finnish = 7,
    French = 8,
    German = 9,
    Greek = 10,
    Hindi = 11,
    Hungarian = 12,
    Indonesian = 13,
    Irish = 14,
    Italian = 15,
    Lithuanian = 16,
    Nepali = 17,
    Norwegian = 18,
    Portuguese = 19,
    Romanian = 20,
    Russian = 21,
    Serbian = 22,
    Simple = 23,
    Spanish = 24,
    Swedish = 25,
    Tamil = 26,
    Turkish = 27,
    Yiddish = 28,
}
