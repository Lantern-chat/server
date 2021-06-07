use db::{Snowflake, SnowflakeExt};

use crate::{
    ctrl::{auth::Authorization, perm::get_room_permissions, Error, SearchMode},
    ServerState,
};

use models::*;

#[derive(Debug, Deserialize)]
pub struct CreateMessageForm {
    content: String,
}

pub async fn create_message(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    form: CreateMessageForm,
) -> Result<Message, Error> {
    let db = state.db.write.get().await?;

    let msg_id = Snowflake::now();

    let row = db
        .query_opt_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                tables! {
                    struct GetRoomPermissions in Lantern {
                        Perm: Type::INT8,
                    }

                    struct AggPerm {
                        Perm: GetRoomPermissions::Perm,
                    }

                    struct AggMsg {
                        UserId: Users::Id,
                        RoomId: Rooms::Id,
                    }
                }

                const CREATE_MESSAGE: i64 = Permission {
                    party: PartyPermissions::empty(),
                    room: RoomPermissions::SEND_MESSAGES,
                    stream: StreamPermissions::empty(),
                }
                .pack() as i64;

                let user_id_var = Var::at(Users::Id, 1);
                let room_id_var = Var::at(Rooms::Id, 2);
                let msg_id_var = Var::at(Messages::Id, 3);
                let content_var = Var::at(Messages::Content, 4);

                let permissions = AggPerm::as_query(
                    Query::select()
                        .expr(GetRoomPermissions::Perm.alias_to(AggPerm::Perm))
                        .from(
                            Call::custom(GetRoomPermissions::full_name())
                                .args((user_id_var.clone(), room_id_var.clone())),
                        ),
                );

                let insert_values = Query::select()
                    .from_table::<AggPerm>()
                    .and_where(
                        AggPerm::Perm
                            .bit_and(Literal::Int8(CREATE_MESSAGE))
                            .equals(Literal::Int8(CREATE_MESSAGE)),
                    )
                    .exprs(vec![msg_id_var, user_id_var, room_id_var, content_var]);

                let insert = AggMsg::as_query(
                    Query::with()
                        .with(permissions)
                        .insert()
                        .into::<Messages>()
                        .cols(&[
                            Messages::Id,
                            Messages::UserId,
                            Messages::RoomId,
                            Messages::Content,
                        ])
                        .query(insert_values.as_value())
                        .returning(Messages::UserId.alias_to(AggMsg::UserId))
                        .returning(Messages::RoomId.alias_to(AggMsg::RoomId)),
                );

                let roles = Query::select()
                    .expr(Builtin::array_agg(RoleMembers::RoleId))
                    .from(
                        RoleMembers::inner_join_table::<Roles>()
                            .on(RoleMembers::RoleId.equals(Roles::Id)),
                    )
                    .and_where(RoleMembers::UserId.equals(AggMsg::UserId))
                    .and_where(Roles::PartyId.equals(Party::Id));

                Query::with()
                    .with(insert)
                    .select()
                    .col(Party::Id)
                    .col(PartyMember::Nickname)
                    .expr(roles.as_value())
                    .cols(&[
                        Users::Username,
                        Users::Discriminator,
                        Users::Flags,
                        Users::CustomStatus,
                        Users::Biography,
                    ])
                    .from_table::<Users>()
                    .from(
                        PartyMember::right_join(
                            Party::right_join_table::<Rooms>().on(Party::Id.equals(Rooms::PartyId)),
                        )
                        .on(PartyMember::PartyId.equals(Party::Id)),
                    )
                    .and_where(Rooms::Id.equals(AggMsg::RoomId))
                    .and_where(Users::Id.equals(AggMsg::UserId))
                    .and_where(PartyMember::UserId.equals(AggMsg::UserId))
            },
            &[&auth.user_id, &room_id, &msg_id, &form.content],
        )
        .await?;

    let row = match row {
        None => return Err(Error::NotFound),
        Some(row) => row,
    };

    let party_id: Option<Snowflake> = row.try_get(0)?;
    let nickname: Option<String> = row.try_get(1)?;
    let roles: Vec<Snowflake> = row.try_get(2)?;

    Ok(Message {
        id: msg_id,
        party_id,
        room_id,
        member: nickname.map(|nick| PartyMember {
            user: None,
            nick: Some(nick),
            roles,
        }),
        author: User {
            id: auth.user_id,
            username: row.try_get(3)?,
            discriminator: row.try_get(4)?,
            flags: UserFlags::from_bits_truncate(row.try_get(5)?).publicize(),
            avatar_id: None,
            status: row.try_get(6)?,
            bio: row.try_get(7)?,
            email: None,
            preferences: None,
        },
        thread_id: None,
        created_at: time::PrimitiveDateTime::from(msg_id.timestamp())
            .assume_utc()
            .format(time::Format::Rfc3339),
        edited_at: None,
        flags: MessageFlags::empty(),
        content: form.content,
        user_mentions: Vec::new(),
        role_mentions: Vec::new(),
        room_mentions: Vec::new(),
        reactions: Vec::new(),
        attachments: Vec::new(),
        embeds: Vec::new(),
    })
}
