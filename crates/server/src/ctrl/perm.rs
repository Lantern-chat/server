use futures::StreamExt;

use db::pool::Client;

use models::*;

use super::*;

pub async fn get_party_permissions(
    db: &Client,
    user_id: Snowflake,
    party_id: Snowflake,
) -> Result<Permission, Error> {
    let row = db
        .query_one_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .col(Party::OwnerId)
                    .expr(Builtin::bit_or(Roles::Permissions))
                    .from(
                        RoleMembers::right_join(
                            Roles::left_join_table::<Party>().on(Roles::PartyId.equals(Party::Id)),
                        )
                        .on(RoleMembers::RoleId.equals(Roles::Id)),
                    )
                    .and_where(Party::Id.equals(Var::of(Party::Id)))
                    .and_where(
                        RoleMembers::UserId
                            .equals(Var::of(Users::Id))
                            .is_not_false(), // null trickery
                    )
                    .group_by(Party::OwnerId)
            },
            &[&party_id, &user_id],
        )
        .await?;

    let owner_id: Snowflake = row.try_get(0)?;

    if owner_id == user_id {
        return Ok(Permission::ALL);
    }

    let permissions: i64 = row.try_get(1)?;

    if (permissions as u64 & Permission::PACKED_ADMIN) == Permission::PACKED_ADMIN {
        return Ok(Permission::ALL);
    }

    Ok(Permission::unpack(permissions as u64))
}

pub async fn get_room_permissions(
    db: &Client,
    user_id: Snowflake,
    room_id: Snowflake,
    party_permissions: Permission,
) -> Result<Permission, Error> {
    if party_permissions.is_admin() {
        return Ok(party_permissions);
    }

    let row = db
        .query_one_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                let room_id_var = Var::at(Rooms::Id, 1);
                let user_id_var = Var::at(Users::Id, 2);

                tables! {
                    pub struct AggregateOverwrites {
                        UserAllow: Overwrites::Allow,
                        UserDeny: Overwrites::Deny,
                        Allow: Overwrites::Allow,
                        Deny: Overwrites::Deny,
                    }
                }

                let subquery = Query::select()
                    .from(
                        Overwrites::left_join_table::<RoleMembers>()
                            .on(RoleMembers::RoleId.equals(Overwrites::RoleId)),
                    )
                    .expr(
                        If::condition(Overwrites::UserId.is_not_null())
                            .then(Overwrites::Allow)
                            .otherwise(Literal::Int8(0))
                            .alias_to(AggregateOverwrites::UserAllow),
                    )
                    .expr(
                        If::condition(Overwrites::UserId.is_not_null())
                            .then(Overwrites::Deny)
                            .otherwise(Literal::Int8(0))
                            .alias_to(AggregateOverwrites::UserDeny),
                    )
                    .expr(
                        If::condition(Overwrites::UserId.is_null())
                            .then(Overwrites::Allow)
                            .otherwise(Literal::Int8(0))
                            .alias_to(AggregateOverwrites::Allow),
                    )
                    .expr(
                        If::condition(Overwrites::UserId.is_null())
                            .then(Overwrites::Deny)
                            .otherwise(Literal::Int8(0))
                            .alias_to(AggregateOverwrites::Deny),
                    )
                    .and_where(Overwrites::RoomId.equals(room_id_var))
                    .and_where(
                        Overwrites::UserId
                            .equals(user_id_var.clone())
                            .is_not_false(),
                    )
                    .and_where(RoleMembers::UserId.equals(user_id_var).is_not_false());

                Query::with()
                    .with(AggregateOverwrites::as_query(subquery))
                    .select()
                    .cols(&[
                        AggregateOverwrites::UserAllow,
                        AggregateOverwrites::UserDeny,
                        AggregateOverwrites::Allow,
                        AggregateOverwrites::Deny,
                    ])
            },
            &[&room_id, &user_id],
        )
        .await?;

    let allow = row.try_get::<_, i64>(0)? as u64;
    let deny = row.try_get::<_, i64>(1)? as u64;
    let user_allow = row.try_get::<_, i64>(2)? as u64;
    let user_deny = row.try_get::<_, i64>(3)? as u64;

    let mut perms = party_permissions.pack();

    perms &= !deny;
    perms |= allow;

    perms &= !user_deny;
    perms |= user_allow;

    Ok(Permission::unpack(perms))
}
