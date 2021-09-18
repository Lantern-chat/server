use super::*;

thorn::tables! {
    pub struct Host in Lantern {
        Migration: Type::INT8,
        Migrated: Type::TIMESTAMP,
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
        Flags: Type::INT2,
        Discriminator: Type::INT2,
        Username: Type::VARCHAR,
        Email: Type::TEXT,
        Passhash: Type::TEXT,
        CustomStatus: Type::VARCHAR,
        Biography: Type::VARCHAR,
        Preferences: Type::JSONB,
        MfaSecret: Type::BYTEA,
        MfaBackup: Type::BYTEA,
    }

    pub struct UsersFreelist in Lantern {
        Username: Type::VARCHAR,
        Descriminator: Type::INT2,
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

    pub struct UserAvatars in Lantern {
        UserId: Users::Id,
        FileId: Files::Id,
        PartyId: Party::Id,
    }

    pub struct Sessions in Lantern {
        UserId: Users::Id,
        Expires: Type::TIMESTAMP,
        Addr: Type::INET,
        Token: Type::BYTEA,
    }

    pub struct Friendlist in Lantern {
        UserAId: Users::Id,
        UserBId: Users::Id,
        Flags: Type::INT2,
        NoteA: Type::VARCHAR,
        NoteB: Type::VARCHAR
    }

    pub struct Party in Lantern {
        Id: SNOWFLAKE,
        AvatarId: Files::Id,
        OwnerId: Users::Id,
        Flags: Type::INT8,
        DeletedAt: Type::TIMESTAMP,
        Name: Type::VARCHAR,
        Description: Type::TEXT,
    }

    pub struct PartyMember in Lantern {
        PartyId: Party::Id,
        UserId: Users::Id,
        InviteId: Invite::Id,
        JoinedAt: Type::TIMESTAMP,
        Flags: Type::INT2,
        SortOrder: Type::INT2,
        Nickname: Type::VARCHAR,
        CustomStatus: Type::VARCHAR,
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
        AvatarId: Files::Id,
        Permissions: Type::INT8,
        /// Color encoded as a 32-bit integer
        Color: Type::INT4,
        Flags: Type::INT2,
        Name: Type::VARCHAR,
    }

    pub struct RoleMembers in Lantern {
        RoleId: Roles::Id,
        UserId: Users::Id,
    }

    pub struct Emotes in Lantern {
        Id: SNOWFLAKE,
        PartyId: Party::Id,
        FileId: Files::Id,
        AspectRatio: Type::FLOAT4,
        Flags: Type::INT2,
        Name: Type::VARCHAR,
        Alt: Type::VARCHAR,
    }

    pub struct Reactions in Lantern {
        EmoteId: Emotes::Id,
        MsgId: Messages::Id,
        UserIds: SNOWFLAKE_ARRAY,
    }

    pub struct Invite in Lantern {
        Id: SNOWFLAKE,
        Code: Type::INT8,
        PartyId: Party::Id,
        UserId: Users::Id,
        Expires: Type::TIMESTAMP,
        Uses: Type::INT2,
        Description: Type::TEXT,
    }

    pub struct Rooms in Lantern {
        Id: SNOWFLAKE,
        PartyId: SNOWFLAKE,
        AvatarId: SNOWFLAKE,
        ParentId: Rooms::Id,
        DeletedAt: Type::TIMESTAMP,
        SortOrder: Type::INT2,
        Flags: Type::INT2,
        Name: Type::TEXT,
        Topic: Type::VARCHAR,
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

    pub struct Messages in Lantern {
        Id: SNOWFLAKE,
        UserId: Users::Id,
        RoomId: Rooms::Id,
        ThreadId: SNOWFLAKE,
        UpdatedAt: Type::TIMESTAMP,
        EditedAt: Type::TIMESTAMP,
        Flags: Type::INT2,
        Content: Type::TEXT,
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
    }

    pub struct Files in Lantern {
        Id: SNOWFLAKE,
        UserId: Users::Id,
        Nonce: Type::INT8,
        Size: Type::INT4,
        Flags: Type::INT2,
        Name: Type::TEXT,
        Mime: Type::TEXT,
        Sha1: Type::BYTEA,
        Preview: Type::BYTEA,
    }
}
