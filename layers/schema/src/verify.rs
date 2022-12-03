use crate::*;

pub fn iterate_tables_str() -> impl Iterator<Item = String> {
    use thorn::Collectable;

    iterate_tables().map(|q| q.to_string().0)
}

pub fn iterate_tables() -> impl Iterator<Item = thorn::query::SelectQuery> {
    use thorn::table::RealTable;

    macro_rules! impl_iter_tables {
        ($($table:ty,)*) => {
            [$(<$table>::verify()),*].into_iter()
        };
    }

    impl_iter_tables! {
        // Tables
        Host,
        Metrics,
        EventLog,
        EventLogLastNotification,
        RateLimits,
        IpBans,
        Users,
        UserFreelist,
        UserTokens,
        UserPresence,
        UserAssets,
        UserAssetFiles,
        Profiles,
        Sessions,
        Friends,
        UserBlocks,
        Party,
        PartyMember,
        PartyBans,
        Subscriptions,
        Roles,
        RoleMembers,
        Emotes,
        Emojis,
        Reactions,
        Invite,
        Rooms,
        Overwrites,
        DMs,
        GroupMessage,
        GroupMember,
        Threads,
        Messages,
        Mentions,
        Attachments,
        Files,
        PinTags,
        MessagePins,

        // Views
        AggMentions,
        AggPresence,
        AggUsers,
        AggMembers,
        AggMembersFull,
        AggMemberPresence,
        AggAssets,
        AggAttachments,
        AggFriends,
        AggRoomPerms,
        AggUsedFiles,
        AggOriginalProfileFiles,
        AggUserAssociations,
        AggReactions,
    }
}
