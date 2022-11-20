use super::*;

pub const UINT2: Type = Type::INT4;

thorn::tables! {
    pub struct Host in Lantern {
        Migration: Type::INT8,
        Migrated: Type::TIMESTAMP,
    }

    pub struct Metrics in Lantern {
        Ts: Type::TIMESTAMP,

        Mem: Type::INT8,
        Upload: Type::INT8,

        Reqs: Type::INT4,
        Errs: Type::INT4,
        Conns: Type::INT4,
        Events: Type::INT4,

        P50: Type::INT2,
        P95: Type::INT2,
        P99: Type::INT2,
    }

    pub struct EventLog in Lantern {
        /// Incrementing counter for sorting
        Counter: Type::INT8,
        /// Event code
        Code: Type::INT2,
        /// Associated Snowflake for whatever the event points to
        Id: SNOWFLAKE,
        /// If the event is for a party, have this to sort with
        PartyId: SNOWFLAKE,
        /// Rarely, only the room_id will be given
        RoomId: SNOWFLAKE,
    }

    pub struct EventLogLastNotification in Lantern {
        LastNotif: Type::TIMESTAMP,
        MaxInterval: Type::INTERVAL,
    }

    pub struct RateLimits in Lantern {
        Violations: Type::INT4,
        Addr: Type::INET,
    }

    pub struct IpBans in Lantern {
        Expires: Type::TIMESTAMP,
        Addr: Type::INET,
    }

    pub struct Users in Lantern {
        Id: SNOWFLAKE,
        DeletedAt: Type::TIMESTAMP,
        Dob: Type::DATE,
        Flags: Type::INT4,
        Discriminator: UINT2,
        Username: Type::TEXT,
        Email: Type::TEXT,
        Passhash: Type::TEXT,
        Preferences: Type::JSONB,
        MfaSecret: Type::BYTEA,
        MfaBackup: Type::BYTEA,
    }

    pub struct UserFreelist in Lantern {
        Username: Type::TEXT,
        Discriminator: UINT2,
    }

    pub struct UserTokens in Lantern {
        Id: SNOWFLAKE,
        UserId: Users::Id,
        Expires: Type::TIMESTAMP,
        Kind: Type::INT2,
        Token: Type::BYTEA,
    }

    pub struct UserPresence in Lantern {
        UserId: Users::Id,
        ConnId: SNOWFLAKE,
        UpdatedAt: Type::TIMESTAMP,
        Flags: Type::INT2,
        Activity: Type::JSONB,
    }

    pub struct UserAssets in Lantern {
        Id: SNOWFLAKE,
        FileId: Files::Id,
        Preview: Type::BYTEA,
    }

    pub struct UserAssetFiles in Lantern {
        AssetId: UserAssets::Id,
        FileId: Files::Id,
        Flags: Type::INT2,
    }

    pub struct Profiles in Lantern {
        UserId: Users::Id,
        PartyId: Party::Id, // NULLable
        AvatarId: UserAssets::Id,
        BannerId: UserAssets::Id,
        Bits: Type::INT4,
        Extra: Type::INT4,
        Nickname: Type::TEXT,
        CustomStatus: Type::TEXT,
        Biography: Type::TEXT,
    }

    pub struct Sessions in Lantern {
        UserId: Users::Id,
        Expires: Type::TIMESTAMP,
        Addr: Type::INET,
        Token: Type::BYTEA,
    }

    pub struct Friends in Lantern {
        UserAId: Users::Id,
        UserBId: Users::Id,
        UpdatedAt: Type::TIMESTAMP,
        Flags: Type::INT2,
        NoteA: Type::TEXT,
        NoteB: Type::TEXT,
    }

    pub struct UserBlocks in Lantern {
        UserId: Users::Id,
        BlockId: Users::Id,
        BlockedAt: Type::TIMESTAMP,
    }

    pub struct Party in Lantern {
        Id: SNOWFLAKE,
        OwnerId: Users::Id,
        DefaultRoom: Rooms::Id,
        AvatarId: UserAssets::Id,
        BannerId: UserAssets::Id,
        Flags: Type::INT8,
        DeletedAt: Type::TIMESTAMP,
        Name: Type::TEXT,
        Description: Type::TEXT,
    }

    pub struct PartyMember in Lantern {
        PartyId: Party::Id,
        UserId: Users::Id,
        InviteId: Invite::Id,
        JoinedAt: Type::TIMESTAMP,
        Flags: Type::INT2,
        Position: Type::INT2,
    }

    pub struct PartyBans in Lantern {
        PartyId: Party::Id,
        UserId: Users::Id,
        BannedAt: Type::TIMESTAMP,
        Reason: Type::TEXT,
    }

    pub struct Subscriptions in Lantern {
        UserId: Users::Id,
        RoomId: Rooms::Id,
        MuteExpires: Type::TIMESTAMP,
        Flags: Type::INT2,
    }

    pub struct Roles in Lantern {
        Id: SNOWFLAKE,
        PartyId: Party::Id,
        AvatarId: UserAssets::Id,
        Permissions: Type::INT8,
        /// Color encoded as a 32-bit integer
        Color: Type::INT4,
        Position: Type::INT2,
        Flags: Type::INT2,
        Name: Type::TEXT,
    }

    pub struct RoleMembers in Lantern {
        RoleId: Roles::Id,
        UserId: Users::Id,
    }

    pub struct Emotes in Lantern {
        Id: SNOWFLAKE,
        PartyId: Party::Id,
        AssetId: UserAssets::Id,
        AspectRatio: Type::FLOAT4,
        Flags: Type::INT2,
        Name: Type::TEXT,
        Alt: Type::TEXT,
    }

    pub struct Emojis in Lantern {
        Id: Type::INT4,
        Flags: Type::INT2,
        Emoji: Type::TEXT,
        Aliases: Type::TEXT,
        Tags: Type::TEXT,
    }

    pub struct Reactions in Lantern {
        MsgId: Messages::Id,
        EmoteId: Emotes::Id,
        EmojiId: Emojis::Id,
        Reacted: Type::TIMESTAMP,
        UserIds: SNOWFLAKE_ARRAY,
    }

    pub struct Invite in Lantern {
        Id: SNOWFLAKE,
        PartyId: Party::Id,
        UserId: Users::Id,
        Expires: Type::TIMESTAMP,
        Uses: Type::INT2,
        Description: Type::TEXT,
        Vanity: Type::TEXT,
    }

    pub struct Rooms in Lantern {
        Id: SNOWFLAKE,
        PartyId: SNOWFLAKE,
        AvatarId: SNOWFLAKE,
        ParentId: Rooms::Id,
        DeletedAt: Type::TIMESTAMP,
        Position: Type::INT2,
        Flags: Type::INT2,
        Name: Type::TEXT,
        Topic: Type::TEXT,
    }

    pub struct Overwrites in Lantern {
        RoomId: Rooms::Id,
        Allow: Type::INT8,
        Deny: Type::INT8,
        RoleId: Roles::Id,
        UserId: Users::Id,
    }

    pub struct DMs as "dms" in Lantern {
        UserIdA: Users::Id,
        UserIdB: Users::Id,
        RoomId: Rooms::Id,
    }

    pub struct GroupMessage in Lantern {
        Id: SNOWFLAKE,
        RoomId: Rooms::Id,
    }

    pub struct GroupMember in Lantern {
        GroupId: GroupMessage::Id,
        UserId: Users::Id,
    }

    pub struct Threads in Lantern {
        Id: SNOWFLAKE,
        ParentId: Messages::Id,
        Flags: Type::INT2,
    }

    pub struct Messages in Lantern {
        Id: SNOWFLAKE,
        UserId: Users::Id,
        RoomId: Rooms::Id,
        ThreadId: Threads::Id,
        UpdatedAt: Type::TIMESTAMP,
        EditedAt: Type::TIMESTAMP,
        Kind: Type::INT2,
        Flags: Type::INT2,
        Content: Type::TEXT,
        Ts: Type::TS_VECTOR,
        PinTags: SNOWFLAKE_ARRAY,
    }

    pub struct Mentions in Lantern {
        MsgId: Messages::Id,
        UserId: Users::Id,
        RoleId: Roles::Id,
        RoomId: Rooms::Id,
    }

    pub struct Attachments in Lantern {
        MessageId: Messages::Id,
        FileId: Files::Id,
        Flags: Type::INT2,
    }

    pub struct Files in Lantern {
        Id: SNOWFLAKE,
        UserId: Users::Id,
        Nonce: Type::INT8,
        Size: Type::INT4,
        Width: Type::INT4,
        Height: Type::INT4,
        Flags: Type::INT2,
        Name: Type::TEXT,
        Mime: Type::TEXT,
        Sha1: Type::BYTEA,
        Preview: Type::BYTEA,
    }

    pub struct PinTags in Lantern {
        Id: SNOWFLAKE,
        IconId: Emotes::Id,
        Flags: Type::INT4,
        Name: Type::TEXT,
        Description: Type::TEXT,
    }

    pub struct MessagePins in Lantern {
        TagId: PinTags::Id,
        MsgId: Messages::Id,
    }
}
