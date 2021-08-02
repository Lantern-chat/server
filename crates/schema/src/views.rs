use super::*;

thorn::tables! {
    pub struct AggMentions in Lantern {
        Kinds: Type::INT4_ARRAY,
        Ids: SNOWFLAKE_ARRAY,
    }

    pub struct AggMessages in Lantern {
        MsgId: Messages::Id,
        UserId: Messages::UserId,
        PartyId: Rooms::PartyId,
        RoomId: Messages::RoomId,
        Nickname: PartyMember::Nickname,
        Username: Users::Username,
        UserFlags: Users::Flags,
        Discriminator: Users::Discriminator,
        MentionKinds: AggMentions::Kinds,
        MentionIds: AggMentions::Ids,
        EditedAt: Messages::EditedAt,
        MessageFlags: Messages::Flags,
        Content: Messages::Content,
        Roles: SNOWFLAKE_ARRAY,
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

#[derive(Debug, serde::Deserialize)]
pub struct AggAttachmentsMeta {
    pub id: Snowflake,
    pub size: i32,
    pub name: String,
    pub mime: Option<String>,
}
