use crate::prelude::*;
use sdk::api::commands::all::DeleteRoom;
use sdk::models::*;

pub async fn remove_room(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<DeleteRoom>,
) -> Result<(), Error> {
    let room_id: RoomId = cmd.room_id.into();

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    #[rustfmt::skip]
    let res = t.execute2(schema::sql! {
        const_assert!(!Columns::IS_DYNAMIC);

        struct PendingRoom { Id: Rooms::Id }

        WITH PendingRoom AS (
            SELECT Rooms.Id AS PendingRoom.Id
              FROM LiveRooms AS Rooms INNER JOIN PartyMembers ON PartyMembers.PartyId = Rooms.PartyId
             WHERE Rooms.Id = #{&room_id as Rooms::Id}
               AND PartyMembers.UserId = #{auth.user_id_ref() as Users::Id}

            const PERMS: [i64; 2] = Permissions::MANAGE_ROOMS.to_i64();
            const_assert!(PERMS[1] == 0);

            AND PartyMembers.Permissions1 & const {PERMS[0]} = const {PERMS[0]}
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
