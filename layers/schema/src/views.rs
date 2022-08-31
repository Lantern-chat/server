use super::*;

thorn::tables! {
    pub struct AggMentions in Lantern {
        MsgId: Messages::Id,
        Kinds: Type::INT4_ARRAY,
        Ids: SNOWFLAKE_ARRAY,
    }

    pub struct AggUsers in Lantern {
        Id: Users::Id,
        Discriminator: Users::Discriminator,
        Email: Users::Email,
        Flags: Users::Flags,
        Username: Users::Username,
        Preferences: Users::Preferences,
        PresenceFlags: UserPresence::Flags,
        PresenceUpdatedAt: UserPresence::UpdatedAt,
        PresenceActivity: UserPresence::Activity,
    }

    pub struct AggMembers in Lantern {
        UserId: Users::Id,
        PartyId: Party::Id,
        Nickname: PartyMember::Nickname,
        Flags: PartyMember::Flags,
        JoinedAt: PartyMember::JoinedAt,
        RoleIds: SNOWFLAKE_ARRAY,
    }

    pub struct AggMembersFull in Lantern {
        PartyId: Party::Id,
        UserId: Users::Id,
        Discriminator: AggUsers::Discriminator,
        Username: AggUsers::Username,
        UserFlags: AggUsers::Flags,
        PresenceFlags: AggUsers::PresenceFlags,
        PresenceUpdatedAt: AggUsers::PresenceUpdatedAt,
        Nickname: PartyMember::Nickname,
        MemberFlags: PartyMember::Flags,
        JoinedAt: PartyMember::JoinedAt,
        AvatarId: Profiles::AvatarId,
        ProfileBits: Profiles::Bits,
        CustomStatus: Profiles::CustomStatus,
        RoleIds: SNOWFLAKE_ARRAY,
        PresenceActivity: AggUsers::PresenceActivity,
    }

    pub struct AggMemberPresence in Lantern {
        UserId: Users::Id,
        Username: Users::Username,
        Discriminator: Users::Discriminator,
        UserFlags: Users::Flags,
        PartyId: PartyMember::PartyId,
        ProfileBits: Profiles::Bits,
        AvatarId: Profiles::AvatarId,
        BannerId: Profiles::BannerId,
        CustomStatus: Profiles::CustomStatus,
        Biography: Profiles::Biography,
        UpdatedAt: UserPresence::UpdatedAt,
        PresenceFlags: UserPresence::Flags,
        PresenceActivity: UserPresence::Activity,
    }

    pub struct AggAssets in Lantern {
        AssetId: UserAssets::Id,
        AssetFlags: UserAssetFiles::Flags,
        FileId: Files::Id,
        UserId: Files::UserId,
        Nonce: Files::Nonce,
        Size: Files::Size,
        Width: Files::Width,
        Height: Files::Height,
        FileFlags: Files::Flags,
        FileName: Files::Name,
        Mime: Files::Mime,
        Sha1: Files::Sha1,
        Preview: UserAssets::Preview,
    }

    pub struct AggAttachments in Lantern {
        MsgId: Messages::Id,

        /// `Vec<`[`AggAttachmentsMeta`]`>`
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

    pub struct AggUsedFiles in Lantern {
        Id: Files::Id,
    }

    pub struct AggOriginalProfileFiles in Lantern {
        UserId: Users::Id,
        PartyId: Party::Id,
        Bits: Profiles::Bits,
        AvatarFileId: Files::Id,
        BannerFileId: Files::Id,
    }

    pub struct AggUserAssociations in Lantern {
        UserId: Users::Id,
        OtherId: Users::Id,
    }

    pub struct AggReactions in Lantern {
        MsgId: Messages::Id,
        EmoteId: Emotes::Id,
        EmojiId: Emojis::Id,
        Reacted: Type::TIMESTAMP,
        UserIds: SNOWFLAKE_ARRAY,
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

    #[serde(default)]
    pub width: Option<i32>,

    #[serde(default)]
    pub height: Option<i32>,

    #[serde(default)]
    pub flags: Option<i16>,
}
