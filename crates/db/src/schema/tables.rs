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
        /// Incrementing counter for sorting
        Counter: Type::INT8,
        /// Event code
        Code: Type::INT2,
        /// Associated Snowflake for whatever the event points to
        Id: SNOWFLAKE,
        /// If the event is for a party, have this to sort with
        PartyId: SNOWFLAKE,
    }

    pub struct EventLogLastNotification in Lantern {
        LastNotif: Type::TIMESTAMP,
        MaxInterval: Type::INTERVAL,
    }

    pub struct Users in Lantern {
        Id: SNOWFLAKE,
        AvatarId: SNOWFLAKE,
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
    }

    pub struct UserTokens in Lantern {
        Id: SNOWFLAKE,
        UserId: SNOWFLAKE,
        Expires: Type::TIMESTAMP,
        Kind: Type::INT2,
        Token: Type::BYTEA,
    }

    pub struct UserStatus in Lantern {
        UserId: SNOWFLAKE,
        Updated: Type::TIMESTAMP,
        Active: Type::INT2,
    }

    pub struct UsersFreelist in Lantern {
        Username: Type::VARCHAR,
        Descriminator: Type::INT2,
    }

    pub struct Sessions in Lantern {
        UserId: SNOWFLAKE,
        Expires: Type::TIMESTAMP,
        Addr: Type::INET,
        Token: Type::BYTEA,
    }

    pub struct Party in Lantern {
        Id: SNOWFLAKE,
        AvatarId: SNOWFLAKE,
        OwnerId: SNOWFLAKE,
        Flags: Type::INT8,
        DeletedAt: Type::TIMESTAMP,
        Name: Type::VARCHAR,
        Description: Type::TEXT,
    }

    pub struct PartyMember in Lantern {
        PartyId: SNOWFLAKE,
        UserId: SNOWFLAKE,
        InviteId: SNOWFLAKE,
        JoinedAt: Type::TIMESTAMP,
        Flags: Type::INT2,
        Nickname: Type::VARCHAR,
        CustomStatus: Type::VARCHAR,
    }

    pub struct Subscriptions in Lantern {
        UserId: SNOWFLAKE,
        RoomId: SNOWFLAKE,
        MuteExpires: Type::TIMESTAMP,
        Flags: Type::INT2,
    }

    pub struct Roles in Lantern {
        Id: SNOWFLAKE,
        PartyId: SNOWFLAKE,
        Permissions: Type::INT8,
        /// Color encoded as a 32-bit integer
        Color: Type::INT4,
        Flags: Type::INT2,
        Name: Type::VARCHAR,
    }

    pub struct RoleMembers in Lantern {
        RoleId: SNOWFLAKE,
        UserId: SNOWFLAKE,
    }

    pub struct Emotes in Lantern {
        Id: SNOWFLAKE,
        PartyId: SNOWFLAKE,
        FileId: SNOWFLAKE,
        AspectRatio: Type::FLOAT4,
        Flags: Type::INT2,
        Name: Type::VARCHAR,
        Alt: Type::VARCHAR,
    }

    pub struct Reactions in Lantern {
        EmoteId: SNOWFLAKE,
        MsgId: SNOWFLAKE,
        UserIds: SNOWFLAKE_ARRAY,
    }

    pub struct Invite in Lantern {
        Id: SNOWFLAKE,
        PartyId: SNOWFLAKE,
        UserId: SNOWFLAKE,
        Expires: Type::TIMESTAMP,
        Uses: Type::INT2,
        Code: Type::VARCHAR,
        Description: Type::TEXT,
    }

    pub struct Rooms in Lantern {
        Id: SNOWFLAKE,
        PartyId: SNOWFLAKE,
        AvatarId: SNOWFLAKE,
        ParentId: SNOWFLAKE,
        DeletedAt: Type::TIMESTAMP,
        SortOrder: Type::INT2,
        Flags: Type::INT2,
        Name: Type::TEXT,
        Topic: Type::VARCHAR,
    }

    pub struct Overwrites in Lantern {
        RoomId: SNOWFLAKE,
        Allow: Type::INT8,
        Deny: Type::INT8,
        RoleId: SNOWFLAKE,
        UserId: SNOWFLAKE,
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
        Flags: Type::INT2,
        Content: Type::TEXT,
    }

    pub struct Attachments in Lantern {
        MessageId: SNOWFLAKE,
        FileId: SNOWFLAKE,
    }

    pub struct Files in Lantern {
        Id: SNOWFLAKE,
        Size: Type::INT4,
        Offset: Type::INT4,
        Flags: Type::INT2,
        Name: Type::TEXT,
        Mime: Type::TEXT,
        Preview: Type::BYTEA,
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
