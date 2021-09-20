use super::*;

thorn::tables! {
    pub struct AggMentions in Lantern {
        Kinds: Type::INT4_ARRAY,
        Ids: SNOWFLAKE_ARRAY,
    }

    pub struct AggMessages in Lantern {
        MsgId: Messages::Id,
        UserId: Messages::UserId,
        RoomId: Messages::RoomId,
        PartyId: Rooms::PartyId,
        Nickname: PartyMember::Nickname,
        Username: Users::Username,
        Discriminator: Users::Discriminator,
        UserFlags: Users::Flags,
        AvatarId: UserAvatars::FileId,
        MentionKinds: AggMentions::Kinds,
        MentionIds: AggMentions::Ids,
        EditedAt: Messages::EditedAt,
        MessageFlags: Messages::Flags,
        Content: Messages::Content,
        RoleIds: SNOWFLAKE_ARRAY,
        AttachmentMeta: AggAttachments::Meta,
        AttachmentPreview: AggAttachments::Preview,
    }

    pub struct AggUsers in Lantern {
        Id: Users::Id,
        Discriminator: Users::Discriminator,
        Email: Users::Email,
        Flags: Users::Flags,
        Username: Users::Username,
        Biography: Users::Biography,
        CustomStatus: Users::CustomStatus,
        Preferences: Users::Preferences,
        AvatarId: UserAvatars::FileId,
        PresenceFlags: UserPresence::Flags,
        PresenceUpdatedAt: UserPresence::UpdatedAt,
        PresenceActivity: UserPresence::Activity,
    }

    pub struct AggMembers in Lantern {
        UserId: Users::Id,
        PartyId: Party::Id,
        Nickname: PartyMember::Nickname,
        AvatarId: UserAvatars::FileId,
        JoinedAt: PartyMember::JoinedAt,
        RoleIds: SNOWFLAKE_ARRAY,
    }

    pub struct AggAttachments in Lantern {
        MsgId: Messages::Id,

        /// Vec<{id: Snowflake, size: i32, name: String, mime: Option<String>}>
        Meta: Type::JSONB_ARRAY,

        /// Vec<Option<Vec<u8>>>
        Preview: Type::BYTEA_ARRAY,
    }

    pub struct AggFriends in Lantern {
        UserId: Users::Id,
        FriendId: Users::Id,
        Flags: Type::INT2,
        Note: Type::VARCHAR,
    }

    pub struct AggRoomPerms in Lantern {
        RoomId: Overwrites::RoomId,
        UserId: Overwrites::UserId,
        Perms: Type::INT8,
    }
}

use smol_str::SmolStr;

#[derive(Debug, serde::Deserialize)]
pub struct AggAttachmentsMeta {
    pub id: Snowflake,
    pub size: i32,
    pub name: SmolStr,

    #[serde(default)]
    pub mime: Option<SmolStr>,
}
