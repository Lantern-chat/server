thorn::enums! {
    pub enum EventCode in Lantern {
        MessageCreate,
        MessageUpdate,
        MessageDelete,

        /// User started typing in channel
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

        RoleCreated,
        RoleUpdated,
        RoleDeleted,

        InviteCreate,

        MessageReact,
        MessageUnreact,
    }
}
