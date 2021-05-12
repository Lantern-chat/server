use thorn::pg::Type;

use super::*;

thorn::tables! {
    pub struct Host in Lantern {
        Migration: Type::INT8,
        Migrated: Type::TIMESTAMP,
    }

    pub struct Users in Lantern {
        Id: Type::INT8,
        DeletedAt: Type::TIMESTAMP,
        Username: Type::VARCHAR,
        Discriminator: Type::INT2,
        Email: Type::TEXT,
        Dob: Type::DATE,
        Passhash: Type::TEXT,
        Nickname: Type::VARCHAR,
        CustomStatus: Type::VARCHAR,
        Biography: Type::VARCHAR,
        Preferences: Type::JSONB,
        Away: Type::INT2,
        AvatarId: Type::INT8,
        Flags: Type::INT2,
    }

    pub struct Party in Lantern {
        Id: Type::INT8,
        OwnerId: Type::INT8,
        DeletedAt: Type::TIMESTAMP,
        IconId: Type::INT8,
    }

    pub struct PartyMember in Lantern {
        PartyId: Type::INT8,
        UserId: Type::INT8,
        Nickname: Type::VARCHAR,
        Away: Type::INT2,
        AvatarId: Type::INT8,
        InviteId: Type::INT8,
        JoinedAt: Type::TIMESTAMP,
    }

    pub struct Rooms in Lantern {
        Id: Type::INT8,
        PartyId: Type::INT8,
        Name: Type::TEXT,
        Topic: Type::VARCHAR,
        DeletedAt: Type::TIMESTAMP,
        Flags: Type::INT2,
        AvatarId: Type::INT8,
        SortOrder: Type::INT2,
        ParentId: Type::INT8,
    }

    pub struct DMs as "dms" in Lantern {
        UserIdA: Type::INT8,
        UserIdB: Type::INT8,
        RoomId: Type::INT8,
    }

    pub struct GroupMessage in Lantern {
        Id: Type::INT8,
        RoomId: Type::INT8,
    }

    pub struct GroupMember in Lantern {
        GroupId: Type::INT8,
        UserId: Type::INT8,
    }

    pub struct Messages in Lantern {
        Id: Type::INT8,
        UserId: Type::INT8,
        RoomId: Type::INT8,
        ThreadId: Type::INT8,
        UpdatedAt: Type::TIMESTAMP,
        EditedAt: Type::TIMESTAMP,
        DeletedAt: Type::TIMESTAMP,
        Content: Type::TEXT,
        Pinned: Type::BOOL,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use thorn::*;

    #[test]
    fn test_query() {
        let query = Query::select()
            .from_table::<Users>()
            .cols(cols![Users::Id, Users::Username, Users::Discriminator])
            .expr(Builtin::coalesce((PartyMember::Nickname, Users::Nickname)))
            .col(Users::Id)
            .join_left_table_on::<PartyMember, _>(Users::Id.equals(PartyMember::UserId))
            .and_where(PartyMember::PartyId.equals(Var::of(Type::INT8)))
            .to_string();

        println!("{}", query.0);
    }
}
