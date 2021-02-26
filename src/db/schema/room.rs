use crate::db::{Client, ClientError, Snowflake};

bitflags::bitflags! {
    pub struct RoomFlags: i16 {
        const NSFW    = 1 << 0;
        const PRIVATE = 1 << 1;
        const DIRECT  = 1 << 2;
    }
}

serde_shims::impl_serde_for_bitflags!(RoomFlags);

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

impl Room {
    pub async fn of_party(
        client: &Client,
        party_id: Snowflake,
    ) -> Result<impl Iterator<Item = Room>, ClientError> {
        let rows = client.query_cached(
            || "SELECT id, name, topic, avatar_id, sort_order, flags, parent_id FROM lantern.rooms WHERE party_id = $1",
            &[&party_id]
        ).await?;

        Ok(rows.into_iter().map(move |row| Room {
            id: row.get(0),
            party_id,
            name: row.get(1),
            topic: row.get(2),
            avatar_id: row.get(3),
            sort_order: row.get(4),
            flags: RoomFlags::from_bits_truncate(row.get(5)),
            parent_id: row.get(6),
        }))
    }
}
