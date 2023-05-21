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
            FROM Ow INNER JOIN LiveUsers ON LiveUsers.Id = Ow.Id

            UNION ALL // at least one branch has it

            SELECT NULL, Ow.Id, #{&room_id as Rooms::Id},
                NULLIF(Ow.Allow1, 0), NULLIF(Ow.Allow2, 0),
                NULLIF(Ow.Deny1, 0),  NULLIF(Ow.Deny2, 0)
            FROM Ow INNER JOIN Roles ON Roles.Id = Ow.Id
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
    pub fn new(ows: impl IntoIterator<Item = Overwrite>) -> Self {
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
