use crate::prelude::*;

use sdk::api::commands::party::{CreateRoom, CreateRoomKind};
use sdk::models::*;

use crate::internal::role_overwrites::RawOverwrites;

pub async fn create_room(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<CreateRoom>,
) -> Result<FullRoom, Error> {
    let party_id: PartyId = cmd.party_id.into();
    let form = &cmd.body;

    let config = state.config_full();

    if matches!(form.topic.as_deref(), Some(topic) if !config.shared.room_topic_length.contains(&topic.len())) {
        return Err(Error::InvalidTopic);
    }

    let name = schema::names::slug_name(&form.name);

    if !config.shared.room_name_length.contains(&name.len()) {
        return Err(Error::InvalidName);
    }

    // check permissions AND check for the room limit at the same time.
    #[rustfmt::skip]
    let Some(row) = state.db.read.get().await?.query_opt2(schema::sql! {
        const_assert!(!Columns::IS_DYNAMIC);

        SELECT
            COUNT(Rooms.Id)::int4 AS @TotalRooms,
            COUNT(CASE WHEN Rooms.DeletedAt IS NULL THEN Rooms.Id ELSE NULL END)::int4 AS @LiveRooms
        FROM PartyMembers INNER JOIN Rooms ON Rooms.PartyId = PartyMembers.PartyId
        WHERE PartyMembers.PartyId = #{&party_id as Party::Id}
        AND PartyMembers.UserId = #{auth.user_id_ref() as Users::Id}

        const PERMS: [i64; 2] = Permissions::MANAGE_ROOMS.to_i64();
        const_assert!(PERMS[1] == 0);

        AND PartyMembers.Permissions1 & const {PERMS[0]} = const {PERMS[0]}
    }).await? else {
        return Err(Error::Unauthorized);
    };

    let total_rooms: i32 = row.total_rooms()?;
    let live_rooms: i32 = row.live_rooms()?;

    if total_rooms >= config.shared.max_total_rooms as i32 || live_rooms >= config.shared.max_active_rooms as i32 {
        // TODO: Better error message here
        return Err(Error::Unauthorized);
    }

    #[rustfmt::skip]
    let flags = RoomFlags::empty() | match form.kind {
        CreateRoomKind::Text => RoomFlags::from(RoomKind::Text),
        CreateRoomKind::Voice => RoomFlags::from(RoomKind::Voice),
        CreateRoomKind::UserForum => RoomFlags::from(RoomKind::UserForum),
    };

    let raw = RawOverwrites::new(form.overwrites.deserialize_simple().expect("Unable to deserialize overwrites"));
    let room_id = state.sf.gen();

    let mut db = state.db.write.get().await?;

    let t = db.transaction().await?;

    #[rustfmt::skip]
    let num_inserted = t.execute2(schema::sql! {
        struct Ow {
            Id: SNOWFLAKE,
            Allow1: Type::INT8,
            Allow2: Type::INT8,
            Deny1:  Type::INT8,
            Deny2:  Type::INT8,
        }

        WITH Ow AS (
            SELECT
                UNNEST(#{&raw.id as SNOWFLAKE_ARRAY}) AS Ow.Id,
                NULLIF(UNNEST(#{&raw.a1 as Type::INT8_ARRAY}), 0) AS Ow.Allow1,
                NULLIF(UNNEST(#{&raw.a2 as Type::INT8_ARRAY}), 0) AS Ow.Allow2,
                NULLIF(UNNEST(#{&raw.d1 as Type::INT8_ARRAY}), 0) AS Ow.Deny1,
                NULLIF(UNNEST(#{&raw.d2 as Type::INT8_ARRAY}), 0) AS Ow.Deny2
        )
        INSERT INTO Overwrites (UserId, RoleId, RoomId, Allow1, Allow2, Deny1, Deny2) (
            SELECT Ow.Id, NULL, #{&room_id as Rooms::Id}, Ow.Allow1, Ow.Allow2, Ow.Deny1, Ow.Deny2
            FROM Ow INNER JOIN PartyMembers ON PartyMembers.UserId = Ow.Id
            WHERE PartyMembers.PartyId = #{&party_id as Party::Id} // validate that given user is within party

            UNION ALL

            SELECT NULL, Ow.Id, #{&room_id as Rooms::Id}, Ow.Allow1, Ow.Allow2, Ow.Deny1, Ow.Deny2
            FROM Ow INNER JOIN Roles ON Roles.Id = Ow.Id
            WHERE Roles.PartyId = #{&party_id as Party::Id} // validate that given role is within the party
        )
    })
    .await?;

    if num_inserted != raw.id.len() as u64 {
        t.rollback().await?;

        // TODO: Better error here
        return Err(Error::BadRequest);
    }

    t.execute2(schema::sql! {
        INSERT INTO Rooms (Id, PartyId, Position, Flags, Name, Topic) VALUES (
            #{&room_id          as Rooms::Id},
            #{&party_id         as Party::Id},
            #{&form.position    as Rooms::Position},
            #{&flags            as Rooms::Flags},
            #{&name             as Rooms::Name},
            #{&form.topic       as Rooms::Topic}
        )
    })
    .await?;

    t.commit().await?;

    // TODO: should really reuse the db conn, but this api is called so infrequently that I don't care
    crate::internal::get_rooms::get_room(state, auth, room_id).await
}
