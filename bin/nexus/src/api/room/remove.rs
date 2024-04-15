use crate::prelude::*;
use sdk::models::*;

pub async fn remove_room(state: ServerState, auth: Authorization, room_id: RoomId) -> Result<(), Error> {
    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    #[rustfmt::skip]
    let res = t.execute2(schema::sql! {
        tables! { struct PendingRoom { Id: Rooms::Id } };

        WITH PendingRoom AS (
            SELECT Rooms.Id AS PendingRoom.Id
              FROM LiveRooms AS Rooms INNER JOIN PartyMembers ON PartyMembers.PartyId = Rooms.PartyId
             WHERE Rooms.Id = #{&room_id as Rooms::Id}
               AND PartyMembers.UserId = #{auth.user_id_ref() as Users::Id}

            let perms = Permissions::MANAGE_ROOMS.to_i64();
            assert_eq!(perms[1], 0);
            AND PartyMembers.Permissions1 & {perms[0]} = {perms[0]}
        )
        UPDATE Rooms SET (DeletedAt) = now()
        FROM PendingRoom WHERE Rooms.Id = PendingRoom.Id
    }).await?;

    if res != 1 {
        t.rollback().await?;

        return Err(Error::Unauthorized);
    }

    t.commit().await?;

    Ok(())
}
