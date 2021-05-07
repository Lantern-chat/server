use super::*;

#[derive(Debug, Clone, Copy, Iden)]
pub enum Host {
    Table,
    Migration,
    Migrated,
}

#[derive(Debug, Clone, Copy, Iden)]
pub enum Users {
    Table,
    Id,
    DeletedAt,
    Username,
    Discriminator,
    Email,
    Dob,
    Flags,
    AvatarId,
    Passhash,
    Nickname,
    CustomStatus,
    Biography,
    Preferences,
    Away,
}

#[derive(Debug, Clone, Copy, Iden)]
pub enum Party {
    Table,
    Id,
    OwnerId,
    DeletedAt,
    IconId,
}

#[derive(Debug, Clone, Copy, Iden)]
pub enum PartyMember {
    Table,
    UserId,
    Nickname,
    Away,
    AvatarId,
    InviteId,
    JoinedAt,
}

#[derive(Debug, Clone, Copy, Iden)]
pub enum Rooms {
    Table,
    Id,
    PartyId,
    Name,
    Topic,
    DeletedAt,
    Flags,
    AvatarId,
    SortOrder,
    ParentId,
}

#[derive(Debug, Clone, Copy, Iden)]
pub enum DMs {
    #[iden = "dms"]
    Table,
    UserA,
    UserB,
    ChannelId,
}

#[derive(Debug, Clone, Copy, Iden)]
pub enum GroupMessage {
    Table,
    Id,
    ChannelId,
}

#[derive(Debug, Clone, Copy, Iden)]
pub enum GroupMember {
    Table,
    GroupId,
    UserId,
}

#[derive(Debug, Clone, Copy, Iden)]
pub enum Avatar {
    Id,
    FileId,
}

#[cfg(test)]
mod test {
    use super::*;

    // "SELECT id, username, discriminator, is_verified, COALESCE(party_member.nickname, users.nickname), custom_status, biography, COALESCE(party_member.avatar_id, users.avatar_id) FROM users LEFT JOIN party_member ON id = user_id WHERE party_id = $1"

    #[test]
    fn test_query() {
        let query = Query::select()
            .columns(cols![Users::Id, Users::Username, Users::Discriminator])
            .expr(Expr::col(PartyMember::Nickname).if_null(Expr::col(Users::Nickname)))
            .columns(cols![Users::Id])
            .from(DMs::Table)
            .left_join(
                Users::Table,
                Expr::col(DMs::UserA).equals(Users::Table, Users::Id),
            )
            .to_owned();

        let s = query.build(PostgresQueryBuilder).0;

        println!("{}", s);
    }
}
