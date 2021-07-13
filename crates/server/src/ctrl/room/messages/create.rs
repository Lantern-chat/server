use schema::{Snowflake, SnowflakeExt};

use crate::{
    ctrl::{auth::Authorization, perm::get_cached_room_permissions, Error, SearchMode},
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
    let permissions = get_cached_room_permissions(&state, auth.user_id, room_id).await?;

    if !permissions.room.contains(RoomPermissions::SEND_MESSAGES) {
        return Err(Error::NotFound);
    }

    let db = state.db.write.get().await?;

    let msg_id = Snowflake::now();

    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                tables! {
                    struct AggMsg {
                        UserId: Users::Id,
                        RoomId: Rooms::Id,
                    }
                }

                let user_id_var = Var::at(Users::Id, 1);
                let room_id_var = Var::at(Rooms::Id, 2);
                let msg_id_var = Var::at(Messages::Id, 3);
                let content_var = Var::at(Messages::Content, 4);

                let insert = AggMsg::as_query(
                    Query::insert()
                        .into::<Messages>()
                        .cols(&[
                            Messages::Id,
                            Messages::UserId,
                            Messages::RoomId,
                            Messages::Content,
                        ])
                        .values(vec![msg_id_var, user_id_var, room_id_var, content_var])
                        .returning(Messages::UserId.alias_to(AggMsg::UserId))
                        .returning(Messages::RoomId.alias_to(AggMsg::RoomId)),
                );

                let roles = Query::select()
                    .expr(Builtin::array_agg(RoleMembers::RoleId))
                    .from(RoleMembers::inner_join_table::<Roles>().on(RoleMembers::RoleId.equals(Roles::Id)))
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
    let roles = row.try_get(2)?;

    Ok(Message {
        id: msg_id,
        party_id,
        room_id,
        member: nickname.map(|nick| PartyMember {
            user: None,
            nick: Some(nick),
            roles,
            presence: None,
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
        created_at: msg_id.format_timestamp(),
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
