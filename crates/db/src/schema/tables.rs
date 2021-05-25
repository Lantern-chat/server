use thorn::pg::Type;

use super::*;

pub const SNOWFLAKE: Type = Type::INT8;
pub const SNOWFLAKE_ARRAY: Type = Type::INT8_ARRAY;

thorn::tables! {
    pub struct Host in Lantern {
        Migration: Type::INT8,
        Migrated: Type::TIMESTAMP,
    }

    pub struct EventLog in Lantern {
        Code: Type::INT2,
        Id: SNOWFLAKE,
    }

    pub struct Users in Lantern {
        Id: SNOWFLAKE,
        DeletedAt: Type::TIMESTAMP,
        Username: Type::VARCHAR,
        Discriminator: Type::INT2,
        Email: Type::TEXT,
        Dob: Type::DATE,
        Passhash: Type::TEXT,
        CustomStatus: Type::VARCHAR,
        Biography: Type::VARCHAR,
        Preferences: Type::JSONB,
        Away: Type::INT2,
        AvatarId: SNOWFLAKE,
        Flags: Type::INT2,
    }

    pub struct UsersFreelist in Lantern {
        Username: Type::VARCHAR,
        Descriminator: Type::INT2,
    }

    pub struct Sessions in Lantern {
        Token: Type::BYTEA,
        UserId: SNOWFLAKE,
        Expires: Type::TIMESTAMP,
    }

    pub struct Party in Lantern {
        Id: SNOWFLAKE,
        Name: Type::VARCHAR,
        Description: Type::TEXT,
        OwnerId: SNOWFLAKE,
        DeletedAt: Type::TIMESTAMP,
        IconId: SNOWFLAKE,
    }

    pub struct PartyMember in Lantern {
        PartyId: SNOWFLAKE,
        UserId: SNOWFLAKE,
        Nickname: Type::VARCHAR,
        Away: Type::INT2,
        AvatarId: SNOWFLAKE,
        InviteId: SNOWFLAKE,
        JoinedAt: Type::TIMESTAMP,
    }

    pub struct Subscriptions in Lantern {
        UserId: SNOWFLAKE,
        RoomId: SNOWFLAKE,
        Mentions: Type::BOOL,
        MuteExpires: Type::TIMESTAMP,
    }

    pub struct Roles in Lantern {
        Id: SNOWFLAKE,
        PartyId: SNOWFLAKE,
        Name: Type::VARCHAR,
        Permissions: Type::INT8,
        /// Color encoded as a 32-bit integer
        Color: Type::INT4,
        Flags: Type::INT2,
    }

    pub struct RoleMembers in Lantern {
        RoleId: SNOWFLAKE,
        UserId: SNOWFLAKE,
    }

    pub struct Emotes in Lantern {
        Id: SNOWFLAKE,
        PartyId: SNOWFLAKE,
        FileId: SNOWFLAKE,
        Name: Type::VARCHAR,
        Alt: Type::VARCHAR,
        Flags: Type::INT2,
        AspectRatio: Type::FLOAT4,
    }

    pub struct Invite in Lantern {
        Id: SNOWFLAKE,
        PartyId: SNOWFLAKE,
        UserId: SNOWFLAKE,
        Code: Type::VARCHAR,
        Description: Type::TEXT,
        Expires: Type::TIMESTAMP,
        Uses: Type::INT2,
    }

    pub struct Rooms in Lantern {
        Id: SNOWFLAKE,
        PartyId: SNOWFLAKE,
        Name: Type::TEXT,
        Topic: Type::VARCHAR,
        DeletedAt: Type::TIMESTAMP,
        Flags: Type::INT2,
        AvatarId: SNOWFLAKE,
        SortOrder: Type::INT2,
        ParentId: SNOWFLAKE,
    }

    pub struct Overwrites in Lantern {
        RoomId: SNOWFLAKE,
        RoleId: SNOWFLAKE,
        UserId: SNOWFLAKE,
        Allow: Type::INT8,
        Deny: Type::INT8,
    }

    pub struct DMs as "dms" in Lantern {
        UserIdA: SNOWFLAKE,
        UserIdB: SNOWFLAKE,
        RoomId: SNOWFLAKE,
    }

    pub struct GroupMessage in Lantern {
        Id: SNOWFLAKE,
        RoomId: SNOWFLAKE,
    }

    pub struct GroupMember in Lantern {
        GroupId: SNOWFLAKE,
        UserId: SNOWFLAKE,
    }

    pub struct Messages in Lantern {
        Id: SNOWFLAKE,
        UserId: SNOWFLAKE,
        RoomId: SNOWFLAKE,
        ThreadId: SNOWFLAKE,
        UpdatedAt: Type::TIMESTAMP,
        EditedAt: Type::TIMESTAMP,
        DeletedAt: Type::TIMESTAMP,
        Content: Type::TEXT,
        Pinned: Type::BOOL,
    }

    pub struct Attachments in Lantern {
        MessageId: SNOWFLAKE,
        FileId: SNOWFLAKE,
    }

    pub struct Files in Lantern {
        Id: SNOWFLAKE,
        Name: Type::TEXT,
        Preview: Type::BYTEA,
        Mime: Type::TEXT,
        Size: Type::INT4,
        Sha3: Type::BYTEA,
        Offset: Type::INT4,
        Flags: Type::INT2,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use thorn::*;

    #[test]
    fn test_query() {
        let query = Query::select()
            .cols(&[Users::Id, Users::Username, Users::Discriminator])
            .from(Users::left_join_table::<PartyMember>().on(Users::Id.equals(PartyMember::UserId)))
            .and_where(PartyMember::PartyId.equals(Var::of(SNOWFLAKE)))
            .to_string();

        println!("{}", query.0);
    }
}
