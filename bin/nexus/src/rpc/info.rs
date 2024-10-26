use rpc::request::PartyInfo;

use crate::prelude::*;

#[derive(Debug, Clone, Copy)]
pub enum InfoRequest {
    GetPartyInfoFromPartyId(PartyId),
    GetPartyInfoFromRoomId(RoomId),
}

pub async fn get_party_info(state: ServerState, req: InfoRequest) -> Result<PartyInfo, Error> {
    let db = state.db.read.get().await?;

    match &req {
        // easy case where we just need to get the room_ids from the party_id
        InfoRequest::GetPartyInfoFromPartyId(party_id) => {
            #[rustfmt::skip]
            let res = db.query_one2(schema::sql! {
                SELECT ARRAY_AGG(Rooms.Id) AS @RoomIds
                FROM Rooms WHERE Rooms.PartyId = #{party_id as Rooms::PartyId}
            }).await?;

            Ok(PartyInfo {
                party_id: *party_id,
                room_ids: res.room_ids()?,
            })
        }

        // get the party_id from the room_id and then get the room_ids from the party_id
        InfoRequest::GetPartyInfoFromRoomId(room_id) => {
            #[rustfmt::skip]
            let res = db.query_one2(schema::sql! {
                struct RoomParty { PartyId: Rooms::PartyId }

                WITH RoomParty AS (
                    SELECT Rooms.PartyId AS RoomParty.PartyId
                    FROM Rooms WHERE Rooms.Id = #{room_id as Rooms::Id}
                )

                SELECT
                    Rooms.PartyId       AS @PartyId,
                    ARRAY_AGG(Rooms.Id) AS @RoomIds
                FROM Rooms WHERE Rooms.PartyId = RoomParty.PartyId
                GROUP BY Rooms.PartyId
            }).await?;

            Ok(PartyInfo {
                party_id: res.party_id()?,
                room_ids: res.room_ids()?,
            })
        }
    }
}
