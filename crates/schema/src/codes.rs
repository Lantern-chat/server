thorn::enums! {
    pub enum EventCode in Lantern {
        MessageCreate,
        MessageUpdate,
        MessageDelete,

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
        /// Member was banned, only sent if proper intent was used
        MemberBan,
        /// Member was unbanned, only sent if proper intent was used
        MemberUnban,

        RoleCreated,
        RoleUpdated,
        RoleDeleted,

        InviteCreate,

        MessageReact,
        MessageUnreact,
    }
}
