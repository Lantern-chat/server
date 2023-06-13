use std::collections::HashMap;

use schema::Snowflake;

use crate::{Authorization, Error, ServerState};

use sdk::models::*;

#[derive(Default, Debug, Clone, Deserialize)]
pub struct StatsForm {
    #[serde(default)]
    pub room_id: Option<Snowflake>,
    #[serde(default)]
    pub user_id: Option<Snowflake>,
    #[serde(default)]
    pub prefix: Option<SmolStr>,
}

pub async fn get_stats(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
    form: StatsForm,
) -> Result<Statistics, Error> {
    #[rustfmt::skip]
    let rows = state.db.read.get().await?.query2(schema::sql! {
        tables! {
            struct AllowedRooms {
                RoomId: SNOWFLAKE_ARRAY,
            }
        };

        WITH AllowedRooms AS (
            SELECT AggRoomPerms.Id AS AllowedRooms.RoomId
            FROM   AggRoomPerms
            WHERE  AggRoomPerms.UserId = #{&auth.user_id as Users::Id}
            AND (
                // we know this perm is in the lower half, so only use that
                let perms = Permissions::READ_MESSAGE_HISTORY.to_i64();
                assert_eq!(perms[1], 0);

                AggRoomPerms.Permissions1 & {perms[0]} = {perms[0]}
            )

            AND match form.room_id {
                Some(ref room_id) => { AggRoomPerms.Id      = #{room_id as Rooms::Id} }
                None              => { AggRoomPerms.PartyId = #{&party_id as Party::Id} },
            }
        )
        SELECT
            Messages.RoomId AS @RoomId,

            COUNT(Messages.Id)::int8 AS @Total,

            match form.prefix {
                Some(ref prefix) => { COUNT(
                    CASE WHEN lower(Messages.Content) SIMILAR TO #{prefix as Type::TEXT} THEN 1::int8 ELSE NULL END
                ) },
                None => { 0::int8 },
            } AS @Prefixed,

            SUM((
                SELECT COUNT(Attachments.FileId)::int8
                FROM Attachments
                WHERE Attachments.MsgId = Messages.Id
            )) AS @FileCount

        FROM Messages INNER JOIN AllowedRooms ON Messages.RoomId = AllowedRooms.RoomId

        if let Some(ref user_id) = form.user_id {
            INNER JOIN AggRelationships
                ON AggRelationships.UserId = Messages.UserId
                AND AggRelationships.FriendId = #{user_id as Users::Id}
        }

        // always only count non-deleted
        WHERE Messages.Flags & {MessageFlags::DELETED.bits()} = 0

        if let Some(ref user_id) = form.user_id {
            // don't count requests from dangerous users
            AND (AggRelationships.RelA = {UserRelationship::BlockedDangerous as i8}) IS NOT TRUE
            AND Messages.UserId = #{user_id as Users::Id}
        }

        GROUP BY Messages.RoomId
    }).await?;

    let mut rooms = HashMap::default();

    for row in rows {
        rooms.insert(
            row.room_id()?,
            RoomStatistics {
                messages: row.total::<i64>()? as u64,
                files: row.file_count::<i64>()? as u64,
                prefixed: row.prefixed::<i64>()? as u64,
            },
        );
    }

    Ok(Statistics { rooms })
}
