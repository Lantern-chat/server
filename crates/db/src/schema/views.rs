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
        Discriminator: Users::Discriminator,
        MentionKinds: AggMentions::Kinds,
        MentionIds: AggMentions::Ids,
        EditedAt: Messages::EditedAt,
        Flags: Messages::Flags,
        Content: Messages::Content,
    }

    pub struct AggFriends in Lantern {
        UserId: Users::Id,
        FriendId: Users::Id,
        Flags: Type::INT2,
        Note: Type::VARCHAR,
    }
}
