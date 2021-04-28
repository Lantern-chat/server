use super::{Client, ClientError, Snowflake};

pub use models::room::RoomFlags;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Snowflake,
    pub party_id: Snowflake,
    pub name: String,
    pub topic: Option<String>,
    pub avatar_id: Option<Snowflake>,
    pub sort_order: i16,
    pub flags: RoomFlags,
    pub parent_id: Option<Snowflake>,
}

use futures::{StreamExt, TryStreamExt};

impl Room {
    pub async fn insert(&self, client: &Client) -> Result<(), ClientError> {
        client.write
            .execute_cached(
                || "INSERT INTO lantern.rooms (id, party_id, name, topic, avatar_id, sort_order, flags, parent_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                &[&self.id, &self.party_id, &self.name, &self.topic, &self.avatar_id, &self.sort_order, &self.flags.bits(), &self.parent_id],
            )
            .await?;

        Ok(())
    }

    pub async fn find(client: &Client, room_id: Snowflake) -> Result<Option<Room>, ClientError> {
        let row = client.read.query_opt_cached(
            || "SELECT party_id, name, topic, avatar_id, sort_order, flags, parent_id FROM lantern.rooms WHERE id = $1",
            &[&room_id]
        ).await?;

        Ok(match row {
            None => None,
            Some(row) => Some(Room {
                id: room_id,
                party_id: row.try_get(0)?,
                name: row.try_get(1)?,
                topic: row.try_get(2)?,
                avatar_id: row.try_get(3)?,
                sort_order: row.try_get(4)?,
                flags: RoomFlags::from_bits_truncate(row.try_get(5)?),
                parent_id: row.try_get(6)?,
            }),
        })
    }

    pub async fn of_party(client: &Client, party_id: Snowflake) -> Result<Vec<Room>, ClientError> {
        client.read.query_stream_cached(
            || "SELECT id, name, topic, avatar_id, sort_order, flags, parent_id FROM lantern.rooms WHERE party_id = $1",
            &[&party_id]
        )
        .await?
        .map(|res| match res {
            Err(e) => Err(e),
            Ok(row) => Ok(Room {
                id: row.try_get(0)?,
                party_id,
                name: row.try_get(1)?,
                topic: row.try_get(2)?,
                avatar_id: row.try_get(3)?,
                sort_order: row.try_get(4)?,
                flags: RoomFlags::from_bits_truncate(row.try_get(5)?),
                parent_id: row.try_get(6)?,
            })
        })
        .try_collect()
        .await
    }
}
