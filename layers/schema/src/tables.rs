//! Autogenerated Schema for "lantern"

use thorn::{enums::EnumType, pg::Type, table::Nullable};

thorn::functions! {
    pub extern "pg" fn array_diff(lhs: Type::ANYARRAY, rhs: Type::ANYARRAY) in Lantern;

    pub extern "pg" fn array_uniq(arr: Type::ANYARRAY) in Lantern;

    pub extern "pg" fn combine_profile_bits(base_bits: Type::INT4, party_bits: Type::INT4, party_avatar: Type::INT8) in Lantern;

    pub extern "pg" fn create_thread(_thread_id: Type::INT8, _parent_id: Type::INT8, _new_flags: Type::INT2) in Lantern;

    pub extern "pg" fn redeem_invite(_user_id: Type::INT8, _invite_id: Type::INT8, _invite_code: Type::TEXT) in Lantern;

    pub extern "pg" fn refresh_all_permissions() in Lantern;

    pub extern "pg" fn register_user(_id: Type::INT8, _username: Type::TEXT, _email: Type::TEXT, _passhash: Type::TEXT, _dob: Type::DATE) in Lantern;

    pub extern "pg" fn set_presence(_user_id: Type::INT8, _conn_id: Type::INT8, _flags: Type::INT2, _activity: Type::JSONB) in Lantern;

    pub extern "pg" fn soft_delete_user(_user_id: Type::INT8, _new_username: Type::TEXT) in Lantern;

    /// Converts a language code into the equivalent regconfig language
    pub extern "pg" fn to_language(__arg0: Type::INT2) in Lantern;

    pub extern "pg" fn update_user(_id: Type::INT8, _username: Type::TEXT, _email: Type::TEXT, _passhash: Type::TEXT) in Lantern;

    pub extern "pg" fn upsert_msg(_id: Type::INT8, _user_id: Type::INT8, _room_id: Type::INT8, _thread_id: Type::INT8, _editor_id: Type::INT8, _updated_at: Type::TIMESTAMPTZ, _deleted_at: Type::TIMESTAMPTZ, _content: Type::TEXT, _pinned: Type::BOOL) in Lantern;

}

thorn::enums! {
    pub enum EventCode in Lantern {
        MessageCreate,
        MessageUpdate,
        MessageDelete,
        TypingStarted,
        UserUpdated,
        SelfUpdated,
        PresenceUpdated,
        PartyCreate,
        PartyUpdate,
        PartyDelete,
        RoomCreated,
        RoomUpdated,
        RoomDeleted,
        MemberUpdated,
        MemberJoined,
        MemberLeft,
        MemberBan,
        MemberUnban,
        RoleCreated,
        RoleUpdated,
        RoleDeleted,
        InviteCreate,
        MessageReact,
        MessageUnreact,
        ProfileUpdated,
        RelUpdated,
    }
}

lazy_static::lazy_static! {
    /// See [EventCode] for full documentation
    pub static ref EVENT_CODE: Type = <EventCode as EnumType>::ty(37462);
}

thorn::tables! {
    pub struct AggAssets in Lantern {
        AssetId: Nullable(Type::INT8),
        AssetFlags: Nullable(Type::INT2),
        FileId: Nullable(Type::INT8),
        UserId: Nullable(Type::INT8),
        Nonce: Nullable(Type::INT8),
        Size: Nullable(Type::INT4),
        Width: Nullable(Type::INT4),
        Height: Nullable(Type::INT4),
        FileFlags: Nullable(Type::INT2),
        FileName: Nullable(Type::TEXT),
        Mime: Nullable(Type::TEXT),
        Sha1: Nullable(Type::BYTEA),
        Preview: Nullable(Type::BYTEA),
    }

    pub struct AggAttachments in Lantern {
        MsgId: Nullable(Type::INT8),
        Meta: Nullable(Type::JSONB),
        Preview: Nullable(Type::BYTEA_ARRAY),
    }

    pub struct AggBroadcastVisibility in Lantern {
        UserId: Nullable(Type::INT8),
        OtherId: Nullable(Type::INT8),
        PartyId: Nullable(Type::INT8),
    }

    pub struct AggMemberPresence in Lantern {
        UserId: Nullable(Type::INT8),
        Discriminator: Nullable(Type::INT4),
        Username: Nullable(Type::TEXT),
        UserFlags: Nullable(Type::INT4),
        PartyId: Nullable(Type::INT8),
        ProfileBits: Nullable(Type::INT4),
        Nickname: Nullable(Type::TEXT),
        AvatarId: Nullable(Type::INT8),
        BannerId: Nullable(Type::INT8),
        CustomStatus: Nullable(Type::TEXT),
        Biography: Nullable(Type::TEXT),
        UpdatedAt: Nullable(Type::TIMESTAMPTZ),
        PresenceFlags: Nullable(Type::INT2),
        PresenceActivity: Nullable(Type::JSONB),
    }

    pub struct AggMembers in Lantern {
        UserId: Nullable(Type::INT8),
        PartyId: Nullable(Type::INT8),
        Flags: Nullable(Type::INT2),
        JoinedAt: Nullable(Type::TIMESTAMPTZ),
        RoleIds: Nullable(Type::INT8_ARRAY),
    }

    pub struct AggMembersFull in Lantern {
        PartyId: Nullable(Type::INT8),
        UserId: Nullable(Type::INT8),
        Discriminator: Nullable(Type::INT4),
        UserFlags: Nullable(Type::INT4),
        LastActive: Nullable(Type::TIMESTAMPTZ),
        Username: Nullable(Type::TEXT),
        PresenceFlags: Nullable(Type::INT2),
        PresenceUpdatedAt: Nullable(Type::TIMESTAMPTZ),
        MemberFlags: Nullable(Type::INT2),
        JoinedAt: Nullable(Type::TIMESTAMPTZ),
        Position: Nullable(Type::INT2),
        ProfileBits: Nullable(Type::INT4),
        AvatarId: Nullable(Type::INT8),
        BannerId: Nullable(Type::INT8),
        Nickname: Nullable(Type::TEXT),
        CustomStatus: Nullable(Type::TEXT),
        Biography: Nullable(Type::TEXT),
        RoleIds: Nullable(Type::INT8_ARRAY),
        PresenceActivity: Nullable(Type::JSONB),
    }

    pub struct AggMentions in Lantern {
        MsgId: Nullable(Type::INT8),
        Kinds: Nullable(Type::INT4_ARRAY),
        Ids: Nullable(Type::INT8_ARRAY),
    }

    pub struct AggOriginalProfileFiles in Lantern {
        UserId: Nullable(Type::INT8),
        PartyId: Nullable(Type::INT8),
        Bits: Nullable(Type::INT4),
        AvatarFileId: Nullable(Type::INT8),
        BannerFileId: Nullable(Type::INT8),
    }

    pub struct AggOverwrites in Lantern {
        RoomId: Nullable(Type::INT8),
        UserId: Nullable(Type::INT8),
        RoleId: Nullable(Type::INT8),
        UserAllow1: Nullable(Type::INT8),
        UserAllow2: Nullable(Type::INT8),
        UserDeny1: Nullable(Type::INT8),
        UserDeny2: Nullable(Type::INT8),
        Allow1: Nullable(Type::INT8),
        Allow2: Nullable(Type::INT8),
        Deny1: Nullable(Type::INT8),
        Deny2: Nullable(Type::INT8),
    }

    /// Returns the single most recent/priority presence
    pub struct AggPresence in Lantern {
        UserId: Nullable(Type::INT8),
        Flags: Nullable(Type::INT2),
        UpdatedAt: Nullable(Type::TIMESTAMPTZ),
        Activity: Nullable(Type::JSONB),
    }

    pub struct AggReactions in Lantern {
        Id: Nullable(Type::INT8),
        MsgId: Nullable(Type::INT8),
        Count: Nullable(Type::INT8),
        EmoteId: Nullable(Type::INT8),
        EmojiId: Nullable(Type::INT4),
    }

    pub struct AggRelationships in Lantern {
        UserId: Nullable(Type::INT8),
        FriendId: Nullable(Type::INT8),
        UpdatedAt: Nullable(Type::TIMESTAMPTZ),
        RelA: Nullable(Type::INT4),
        RelB: Nullable(Type::INT4),
        Note: Nullable(Type::TEXT),
    }

    pub struct AggRoomPerms in Lantern {
        PartyId: Nullable(Type::INT8),
        RoomId: Nullable(Type::INT8),
        UserId: Nullable(Type::INT8),
        Permissions1: Nullable(Type::INT8),
        Permissions2: Nullable(Type::INT8),
    }

    pub struct AggUsedFiles in Lantern {
        Id: Nullable(Type::INT8),
    }

    pub struct AggUserAssociations in Lantern {
        UserId: Nullable(Type::INT8),
        OtherId: Nullable(Type::INT8),
        PartyId: Nullable(Type::INT8),
    }

    pub struct AggUsers in Lantern {
        Id: Nullable(Type::INT8),
        Discriminator: Nullable(Type::INT4),
        Email: Nullable(Type::TEXT),
        Flags: Nullable(Type::INT4),
        LastActive: Nullable(Type::TIMESTAMPTZ),
        Username: Nullable(Type::TEXT),
        Preferences: Nullable(Type::JSONB),
        PresenceFlags: Nullable(Type::INT2),
        PresenceUpdatedAt: Nullable(Type::TIMESTAMPTZ),
        PresenceActivity: Nullable(Type::JSONB),
    }

    pub struct Attachments in Lantern {
        MessageId: Type::INT8,
        FileId: Type::INT8,
        Flags: Nullable(Type::INT2),
    }

    pub struct Dms in Lantern {
        UserIdA: Type::INT8,
        UserIdB: Type::INT8,
        RoomId: Type::INT8,
    }

    pub struct Embeds in Lantern {
        Id: Type::INT8,
        Expires: Type::TIMESTAMPTZ,
        Url: Type::TEXT,
        Embed: Type::JSONB,
    }

    pub struct Emojis in Lantern {
        Id: Type::INT4,
        Flags: Type::INT2,
        Emoji: Type::TEXT,
        Description: Nullable(Type::TEXT),
        Aliases: Nullable(Type::TEXT),
        Tags: Nullable(Type::TEXT),
    }

    pub struct Emotes in Lantern {
        Id: Type::INT8,
        PartyId: Nullable(Type::INT8),
        AssetId: Type::INT8,
        AspectRatio: Type::FLOAT4,
        Flags: Type::INT2,
        Name: Type::TEXT,
        Alt: Nullable(Type::TEXT),
    }

    pub struct EventLog in Lantern {
        /// Incrementing counter for sorting
        Counter: Type::INT8,
        /// The snowflake ID of whatever this event is pointing to
        Id: Type::INT8,
        PartyId: Nullable(Type::INT8),
        RoomId: Nullable(Type::INT8),
        UserId: Nullable(Type::INT8),
        Code: EVENT_CODE.clone(),
    }

    /// Notification rate-limiting table
    pub struct EventLogLastNotification in Lantern {
        LastNotif: Type::TIMESTAMPTZ,
        MaxInterval: Type::INTERVAL,
    }

    /// Backing file table for all attachments, avatars and so forth
    pub struct Files in Lantern {
        Id: Type::INT8,
        UserId: Type::INT8,
        /// Encryption Nonce
        Nonce: Nullable(Type::INT8),
        /// Size of file in bytes
        Size: Type::INT4,
        Width: Nullable(Type::INT4),
        Height: Nullable(Type::INT4),
        Flags: Type::INT2,
        /// Filename given at upload
        Name: Type::TEXT,
        /// MIME type
        Mime: Nullable(Type::TEXT),
        /// SHA-1 hash of completed file
        Sha1: Nullable(Type::BYTEA),
        /// blurhash preview (first frame of video if video). this shouldn't
        /// be too large, less than 128 bytes.
        Preview: Nullable(Type::BYTEA),
    }

    pub struct GroupMembers in Lantern {
        GroupId: Type::INT8,
        UserId: Type::INT8,
    }

    pub struct Groups in Lantern {
        Id: Type::INT8,
        RoomId: Type::INT8,
    }

    pub struct Host in Lantern {
        Migration: Type::INT4,
        Migrated: Type::TIMESTAMPTZ,
    }

    pub struct Invite in Lantern {
        Id: Type::INT8,
        PartyId: Type::INT8,
        UserId: Type::INT8,
        Expires: Type::TIMESTAMPTZ,
        Uses: Type::INT4,
        MaxUses: Type::INT4,
        Description: Type::TEXT,
        Vanity: Nullable(Type::TEXT),
    }

    pub struct IpBans in Lantern {
        Expires: Nullable(Type::TIMESTAMPTZ),
        Address: Nullable(Type::INET),
        Network: Nullable(Type::CIDR),
    }

    pub struct Mentions in Lantern {
        MsgId: Type::INT8,
        UserId: Nullable(Type::INT8),
        RoleId: Nullable(Type::INT8),
        RoomId: Nullable(Type::INT8),
    }

    pub struct MessageEmbeds in Lantern {
        MsgId: Type::INT8,
        EmbedId: Type::INT8,
        Position: Type::INT2,
        /// Additional flags for embeds that are specific to the message
        Flags: Nullable(Type::INT2),
    }

    pub struct MessagePins in Lantern {
        MsgId: Type::INT8,
        PinId: Type::INT8,
    }

    pub struct MessageStars in Lantern {
        MsgId: Type::INT8,
        UserId: Type::INT8,
    }

    pub struct Messages in Lantern {
        Id: Type::INT8,
        UserId: Type::INT8,
        RoomId: Type::INT8,
        ThreadId: Nullable(Type::INT8),
        UpdatedAt: Nullable(Type::TIMESTAMPTZ),
        EditedAt: Nullable(Type::TIMESTAMPTZ),
        Kind: Type::INT2,
        Flags: Type::INT2,
        Content: Nullable(Type::TEXT),
        /// autogenerated tsvector using language code in top 6 bits of
        /// message flags
        Ts: Nullable(Type::TS_VECTOR),
    }

    pub struct Metrics in Lantern {
        Ts: Type::TIMESTAMPTZ,
        /// allocated memory usage, in bytes
        Mem: Type::INT8,
        /// bytes uploaded by users since last metric
        Upload: Type::INT8,
        /// requests since last metric
        Reqs: Type::INT4,
        /// errors since last metric
        Errs: Type::INT4,
        /// number of connected gateway users
        Conns: Type::INT4,
        /// number of gateway events since last metric
        Events: Type::INT4,
        /// 50th latency percently
        P50: Type::INT2,
        /// 95th latency percentile
        P95: Type::INT2,
        /// 99th latency percentile
        P99: Type::INT2,
    }

    pub struct Overwrites in Lantern {
        RoomId: Type::INT8,
        Allow1: Type::INT8,
        Allow2: Type::INT8,
        Deny1: Type::INT8,
        Deny2: Type::INT8,
        RoleId: Nullable(Type::INT8),
        UserId: Nullable(Type::INT8),
    }

    pub struct Party in Lantern {
        Id: Type::INT8,
        OwnerId: Type::INT8,
        DefaultRoom: Type::INT8,
        AvatarId: Nullable(Type::INT8),
        BannerId: Nullable(Type::INT8),
        DeletedAt: Nullable(Type::TIMESTAMPTZ),
        Flags: Type::INT4,
        Name: Type::TEXT,
        Description: Nullable(Type::TEXT),
    }

    pub struct PartyBans in Lantern {
        PartyId: Type::INT8,
        UserId: Type::INT8,
        BannedAt: Type::TIMESTAMPTZ,
        Reason: Nullable(Type::TEXT),
    }

    /// Association map between parties and users
    pub struct PartyMembers in Lantern {
        PartyId: Type::INT8,
        UserId: Type::INT8,
        Permissions1: Type::INT8,
        Permissions2: Type::INT8,
        InviteId: Nullable(Type::INT8),
        JoinedAt: Type::TIMESTAMPTZ,
        MuteUntil: Nullable(Type::TIMESTAMPTZ),
        Flags: Type::INT2,
        Position: Type::INT2,
    }

    pub struct PinTags in Lantern {
        Id: Type::INT8,
        PartyId: Type::INT8,
        IconId: Nullable(Type::INT8),
        Flags: Type::INT4,
        Name: Type::TEXT,
        Description: Nullable(Type::TEXT),
    }

    /// Users can have multiple profiles, with one main profile where the
    /// `party_id` is NULL
    pub struct Profiles in Lantern {
        UserId: Type::INT8,
        PartyId: Nullable(Type::INT8),
        AvatarId: Nullable(Type::INT8),
        BannerId: Nullable(Type::INT8),
        Bits: Type::INT4,
        Extra: Nullable(Type::INT4),
        Nickname: Nullable(Type::TEXT),
        CustomStatus: Nullable(Type::TEXT),
        Biography: Nullable(Type::TEXT),
    }

    pub struct RateLimits in Lantern {
        Violations: Type::INT4,
        Addr: Type::INET,
    }

    pub struct ReactionUsers in Lantern {
        ReactionId: Type::INT8,
        UserId: Type::INT8,
    }

    pub struct Reactions in Lantern {
        Id: Type::INT8,
        MsgId: Type::INT8,
        Count: Type::INT8,
        EmoteId: Nullable(Type::INT8),
        EmojiId: Nullable(Type::INT4),
    }

    pub struct Relationships in Lantern {
        UserAId: Type::INT8,
        UserBId: Type::INT8,
        UpdatedAt: Type::TIMESTAMPTZ,
        Relation: Type::INT2,
        NoteA: Nullable(Type::TEXT),
        NoteB: Nullable(Type::TEXT),
    }

    pub struct RoleMembers in Lantern {
        RoleId: Type::INT8,
        UserId: Type::INT8,
    }

    pub struct Roles in Lantern {
        Id: Type::INT8,
        PartyId: Type::INT8,
        AvatarId: Nullable(Type::INT8),
        Permissions1: Type::INT8,
        Permissions2: Type::INT8,
        Color: Nullable(Type::INT4),
        Position: Type::INT2,
        Flags: Type::INT2,
        Name: Type::TEXT,
    }

    /// Table for holding active per-room per-user settings.
    pub struct RoomMembers in Lantern {
        UserId: Type::INT8,
        RoomId: Type::INT8,
        Allow1: Nullable(Type::INT8),
        Allow2: Nullable(Type::INT8),
        Deny1: Nullable(Type::INT8),
        Deny2: Nullable(Type::INT8),
        LastRead: Nullable(Type::INT8),
        WallpaperId: Nullable(Type::INT8),
        /// If NULL, there is no mute
        MuteExpires: Nullable(Type::TIMESTAMPTZ),
        Flags: Type::INT4,
    }

    pub struct Rooms in Lantern {
        Id: Type::INT8,
        PartyId: Nullable(Type::INT8),
        AvatarId: Nullable(Type::INT8),
        ParentId: Nullable(Type::INT8),
        DeletedAt: Nullable(Type::TIMESTAMPTZ),
        Position: Type::INT2,
        Flags: Type::INT2,
        Name: Type::TEXT,
        Topic: Nullable(Type::TEXT),
    }

    pub struct Sessions in Lantern {
        UserId: Type::INT8,
        Expires: Type::TIMESTAMPTZ,
        Addr: Type::INET,
        Token: Type::BYTEA,
    }

    pub struct Threads in Lantern {
        Id: Type::INT8,
        ParentId: Type::INT8,
        Flags: Type::INT2,
    }

    pub struct UserAssetFiles in Lantern {
        AssetId: Type::INT8,
        FileId: Type::INT8,
        Flags: Type::INT2,
    }

    pub struct UserAssets in Lantern {
        Id: Type::INT8,
        /// Original asset before processing
        FileId: Type::INT8,
        /// One single blurhash preview for all versions of this asset
        Preview: Nullable(Type::BYTEA),
    }

    pub struct UserFreelist in Lantern {
        Username: Type::TEXT,
        Discriminator: Type::INT4,
    }

    pub struct UserPresence in Lantern {
        UserId: Type::INT8,
        ConnId: Type::INT8,
        UpdatedAt: Type::TIMESTAMPTZ,
        Flags: Type::INT2,
        Activity: Nullable(Type::JSONB),
    }

    pub struct UserTokens in Lantern {
        Id: Type::INT8,
        UserId: Type::INT8,
        Expires: Type::TIMESTAMPTZ,
        Kind: Type::INT2,
        Token: Type::BYTEA,
    }

    pub struct Users in Lantern {
        Id: Type::INT8,
        DeletedAt: Nullable(Type::TIMESTAMPTZ),
        LastActive: Nullable(Type::TIMESTAMPTZ),
        Dob: Type::DATE,
        Flags: Type::INT4,
        /// 2-byte integer that can be displayed as 4 hex digits, actually
        /// stored as a 4-byte signed integer because Postgres doesn't support
        /// unsigned...
        Discriminator: Type::INT4,
        Username: Type::TEXT,
        Email: Type::TEXT,
        Passhash: Type::TEXT,
        MfaSecret: Nullable(Type::BYTEA),
        MfaBackup: Nullable(Type::BYTEA),
        /// this is for client-side user preferences, which can be stored as
        /// JSON easily enough
        Preferences: Nullable(Type::JSONB),
    }

}
