use schema::Snowflake;

use crate::{Authorization, Error, ServerState};

use sdk::models::*;

pub async fn remove_room(state: ServerState, auth: Authorization, room_id: Snowflake) -> Result<(), Error> {
    #[rustfmt::skip]
    let res = state.db.write.get().await?.execute2(schema::sql! {
        tables! { struct PendingRoom { Id: Rooms::Id } };

        WITH PendingRoom AS (
            SELECT Rooms.Id AS PendingRoom.Id
              FROM LiveRooms AS Rooms INNER JOIN PartyMembers ON PartyMembers.PartyId = Rooms.PartyId
             WHERE Rooms.Id = #{&room_id as Rooms::Id}
               AND PartyMembers.UserId = #{&auth.user_id as Users::Id}

            let perms = Permissions::MANAGE_ROOMS.to_i64();
            assert_eq!(perms[1], 0);
            AND PartyMembers.Permissions1 & {perms[0]} = {perms[0]}
        )
        UPDATE Rooms SET (DeletedAt) = now()
        FROM PendingRoom WHERE Rooms.Id = PendingRoom.Id
    }).await?;

    if res == 0 {
        return Err(Error::Unauthorized);
    }

    Ok(())
}
