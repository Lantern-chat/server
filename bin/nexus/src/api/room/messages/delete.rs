use schema::Snowflake;

use crate::prelude::*;
use sdk::models::*;

pub async fn delete_msg(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> Result<(), Error> {
    let perms = state.perm_cache.get(auth.user_id(), room_id).await;

    if let Some(ref perms) = perms {
        // user cannot view room at all
        if !perms.contains(Permissions::READ_MESSAGE_HISTORY) {
            return Err(Error::Unauthorized);
        }
    }

    #[rustfmt::skip]
    let res = state.db.write.get().await?.execute2(schema::sql! {
        tables! { struct TempPerms { Permissions1: AggRoomPerms::Permissions1 } };

        if perms.is_none() {
            WITH TempPerms AS (
                SELECT AggRoomPerms.Permissions1 AS TempPerms.Permissions1
                  FROM AggRoomPerms
                 WHERE AggRoomPerms.Id = #{&room_id as Rooms::Id}
                   AND AggRoomPerms.UserId = #{auth.user_id_ref() as Users::Id}
            )
        }

        // Update Flags to include the deleted bit
        UPDATE Messages SET (Flags) = (Messages.Flags | CASE WHEN Messages.UserId = #{auth.user_id_ref() as Users::Id} THEN
            // Add REMOVED if not deleted by the author
            {MessageFlags::DELETED.bits()} ELSE {(MessageFlags::DELETED | MessageFlags::REMOVED).bits()} END
        )

        if perms.is_none() { FROM TempPerms } // include CTE if needed

        WHERE Messages.Id = #{&msg_id as Messages::Id}
          AND Messages.Flags & {MessageFlags::DELETED.bits()} = 0 // prevent double updates

        match perms {
            Some(perm) if !perm.contains(Permissions::MANAGE_MESSAGES) => {
                // if they are a known party member and without manage perm
                AND Messages.UserId = #{auth.user_id_ref() as Users::Id}
                // Some(perm) implies membership
            }
            None => {
                let m = Permissions::MANAGE_MESSAGES.to_i64();
                assert_eq!(m[1], 0);

                AND ((
                    // if the user has permissions to manage messages
                    {m[0]} & TempPerms.Permissions1 = {m[0]}
                ) OR (
                    // or they are a valid party member and it's their own message
                    Messages.UserId = #{auth.user_id_ref() as Users::Id}
                    AND TempPerms.Permissions1 IS NOT NULL
                ))
            }
            _ => {} // no additional constraints
        }
    }).await;

    match res {
        Ok(1) => Ok(()),
        Ok(_) => Err(Error::Unauthorized),
        Err(e) => Err(e.into()),
    }
}
