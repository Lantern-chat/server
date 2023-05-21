use futures::FutureExt;

use schema::{Snowflake, SnowflakeExt};
use smallvec::SmallVec;
use thorn::pg::Json;

use crate::backend::{cache::permission_cache::PermMute, util::encrypted_asset::encrypt_snowflake_opt};
use crate::{Authorization, Error, ServerState};

use sdk::api::commands::party::{CreateRoomForm, CreateRoomKind};
use sdk::models::*;

pub async fn create_room(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
    form: CreateRoomForm,
) -> Result<FullRoom, Error> {
    if !state.config().party.roomname_len.contains(&form.name.len()) {
        return Err(Error::InvalidName);
    }

    // check permissions AND check for the room limit at the same time.
    #[rustfmt::skip]
    let Some(row) = state.db.read.get().await?.query_opt2(schema::sql! {
        SELECT
            COUNT(Rooms.Id)::int4 AS @TotalRooms,
            COUNT(CASE WHEN Rooms.DeletedAt IS NULL THEN Rooms.Id ELSE NULL END)::int4 AS @LiveRooms
        FROM PartyMembers INNER JOIN Rooms ON Rooms.PartyId = PartyMembers.PartyId
        WHERE PartyMembers.PartyId = #{&party_id as Party::Id}
        AND PartyMembers.UserId = #{&auth.user_id as Users::Id}

        let perms = Permissions::MANAGE_ROOMS.to_i64();
        assert_eq!(perms[1], 0);

        AND PartyMembers.Permissions1 & {perms[0]} = {perms[0]}
    }).await? else {
        return Err(Error::Unauthorized);
    };

    let total_rooms: i32 = row.total_rooms()?;
    let live_rooms: i32 = row.live_rooms()?;

    let config = state.config();
    if total_rooms >= config.party.max_rooms as i32 || live_rooms >= config.party.max_active_rooms as i32 {
        // TODO: Better error message here
        return Err(Error::Unauthorized);
    }

    #[rustfmt::skip]
    let flags = RoomFlags::empty() | match form.kind {
        CreateRoomKind::Text => RoomFlags::from(RoomKind::Text),
        CreateRoomKind::Voice => RoomFlags::from(RoomKind::Voice),
        CreateRoomKind::UserForum => RoomFlags::from(RoomKind::UserForum),
    };

    let raw = RawOverwrites::new(form.overwrites);
    let room_id = Snowflake::now();

    let mut db = state.db.write.get().await?;

    let t = db.transaction().await?;

    #[rustfmt::skip]
    let num_inserted = t.execute2(schema::sql! {
        tables! {
            struct Ow {
                Id: SNOWFLAKE,
                Allow1: Type::INT8,
                Allow2: Type::INT8,
                Deny1:  Type::INT8,
                Deny2:  Type::INT8,
            }
        };

        WITH Ow AS (
            SELECT
                UNNEST(#{&raw.id as SNOWFLAKE_ARRAY}) AS Ow.Id,
                UNNEST(#{&raw.a1 as Type::INT8_ARRAY}) AS Ow.Allow1,
                UNNEST(#{&raw.a2 as Type::INT8_ARRAY}) AS Ow.Allow2,
                UNNEST(#{&raw.d1 as Type::INT8_ARRAY}) AS Ow.Deny1,
                UNNEST(#{&raw.d2 as Type::INT8_ARRAY}) AS Ow.Deny2
        )
        INSERT INTO Overwrites (UserId, RoleId, RoomId, Allow1, Allow2, Deny1, Deny2) (
            SELECT Ow.Id, NULL, #{&room_id as Rooms::Id},
                NULLIF(Ow.Allow1, 0), NULLIF(Ow.Allow2, 0),
                NULLIF(Ow.Deny1, 0),  NULLIF(Ow.Deny2, 0)
            FROM Ow INNER JOIN PartyMembers ON PartyMembers.UserId = Ow.Id
            WHERE PartyMembers.PartyId = #{&party_id as Party::Id} // validate that given user is within party

            UNION ALL // at least one branch has it

            SELECT NULL, Ow.Id, #{&room_id as Rooms::Id},
                NULLIF(Ow.Allow1, 0), NULLIF(Ow.Allow2, 0),
                NULLIF(Ow.Deny1, 0),  NULLIF(Ow.Deny2, 0)
            FROM Ow INNER JOIN Roles ON Roles.Id = Ow.Id
            WHERE Roles.PartyId = #{&party_id as Party::Id} // validate that given role is within the party
        )
    })
    .await?;

    if num_inserted != raw.id.len() as u64 {
        // TODO: Better error here
        return Err(Error::BadRequest);
    }

    t.execute2(schema::sql! {
        INSERT INTO Rooms (Id, PartyId, Position, Flags, Name) VALUES (
            #{&room_id as Rooms::Id},
            #{&party_id as Party::Id},
            #{&form.position as Rooms::Position},
            #{&flags as Rooms::Flags},
            #{&form.name as Rooms::Name}
        )
    })
    .await?;

    t.commit().await?;

    // should really reuse the db conn, but this api is called so infrequently that I don't care
    crate::backend::api::room::get::get_room(state, auth, room_id).await
}

#[derive(Default)]
pub struct RawOverwrites {
    id: Vec<Snowflake>,
    a1: Vec<i64>,
    a2: Vec<i64>,
    d1: Vec<i64>,
    d2: Vec<i64>,
}

impl RawOverwrites {
    pub fn new(mut ows: ThinVec<Overwrite>) -> Self {
        if ows.len() > 1 {
            ows.sort_unstable_by_key(|ow| ow.id);
            ows.dedup_by_key(|ow| ow.id);
        }

        let mut raw = RawOverwrites::default();

        // collect overwrites in a SoA format that can be sent to the db
        for ow in ows {
            // ignore pointless overwrites
            if ow.allow.is_empty() && ow.deny.is_empty() {
                continue;
            }

            let [a1, a2] = ow.allow.to_i64();
            let [d1, d2] = ow.deny.to_i64();

            raw.id.push(ow.id);
            raw.a1.push(a1);
            raw.a2.push(a2);
            raw.d1.push(d1);
            raw.d2.push(d2);
        }

        raw
    }
}
