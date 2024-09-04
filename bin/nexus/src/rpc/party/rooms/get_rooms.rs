use sdk::models::*;
use thorn::pg::Json;

use crate::prelude::*;
use crate::util::encrypted_asset::encrypt_snowflake_opt;

#[derive(Debug, Clone, Copy)]
pub enum RoomScope {
    Party(PartyId),
    Room(RoomId),
}

pub async fn get_rooms(
    state: ServerState,
    auth: Authorization,
    scope: RoomScope,
) -> Result<impl Stream<Item = Result<FullRoom, Error>>, Error> {
    #[rustfmt::skip]
    let stream = state.db.read.get().await?.query_stream2(schema::sql! {
        SELECT
            Rooms.Id            AS @RoomId,
            Rooms.PartyId       AS @PartyId,
            Rooms.AvatarId      AS @AvatarId,
            Rooms.ParentId      AS @ParentId,
            Rooms.Permissions1  AS @Permissions1,
            Rooms.Permissions2  AS @Permissions2,
            Rooms.Position      AS @Position,
            Rooms.Flags         AS @Flags,
            Rooms.Name          AS @Name,
            Rooms.Topic         AS @Topic,

            (SELECT jsonb_agg(jsonb_build_object(
                "u", Overwrites.UserId,
                "r", Overwrites.RoleId,
                "a1", Overwrites.Allow1,
                "a2", Overwrites.Allow2,
                "d1", Overwrites.Deny1,
                "d2", Overwrites.Deny2
            )) FROM Overwrites WHERE Overwrites.RoomId = Rooms.Id) AS @Overwrites

        FROM AggRoomPerms AS Rooms

        WHERE match scope {
            RoomScope::Party(ref party_id) => { Rooms.PartyId = #{party_id as Rooms::PartyId} },
            RoomScope::Room(ref room_id)   => { Rooms.Id      = #{room_id  as Rooms::Id} }
        }

        AND Rooms.UserId = #{auth.user_id_ref() as Users::Id}

        let perms = Permissions::VIEW_ROOM.to_i64();
        assert_eq!(perms[1], 0);
        AND Rooms.Permissions1 & {perms[0]} = {perms[0]}
    }).await?;

    Ok(stream.map(move |row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => Ok(FullRoom {
            room: Room {
                id: row.room_id()?,
                flags: row.flags()?,
                party_id: row.party_id()?,
                parent_id: row.parent_id()?,
                avatar: encrypt_snowflake_opt(&state, row.avatar_id()?),
                position: row.position()?,
                rate_limit_per_user: None, // TODO
                overwrites: match row.overwrites::<Option<Json<Vec<RawOverwrite>>>>()? {
                    None => ThinVec::new(),
                    Some(Json(raw)) => {
                        let raw: Vec<RawOverwrite> = raw; // force RA to type inference
                        let mut overwrites = ThinVec::with_capacity(raw.len());

                        for ow in raw {
                            overwrites.push(ow.to_overwrite()?);
                        }

                        overwrites
                    }
                },
                name: row.name()?,
                topic: row.topic()?,
            },
            perms: Permissions::from_i64(row.permissions1()?, row.permissions2()?),
        }),
    }))
}

#[derive(Deserialize)]
struct RawOverwrite {
    u: Option<UserId>,
    r: Option<RoleId>,
    a1: Option<i64>,
    a2: Option<i64>,
    d1: Option<i64>,
    d2: Option<i64>,
}

#[allow(clippy::wrong_self_convention)]
impl RawOverwrite {
    pub fn to_overwrite(self) -> Result<Overwrite, Error> {
        let Some(id) = self.r.or(self.u) else {
            return Err(Error::InternalErrorStatic("No ID for Overwrite!"));
        };

        Ok(Overwrite {
            id,
            allow: Permissions::from_i64_opt(self.a1, self.a2),
            deny: Permissions::from_i64_opt(self.d1, self.d2),
        })
    }
}
