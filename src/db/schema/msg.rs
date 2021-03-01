use futures::{Stream, StreamExt, TryStreamExt};

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub room_id: Snowflake,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_id: Option<Snowflake>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<Snowflake>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<time::PrimitiveDateTime>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<time::PrimitiveDateTime>,

    pub content: String,

    #[serde(skip_serializing_if = "crate::db::util::is_false")]
    pub pinned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageSearch {
    After(Snowflake),
    Before(Snowflake),
    Around(Snowflake),
}

impl Message {
    pub async fn find(id: Snowflake, client: &Client) -> Result<Option<Message>, ClientError> {
        let row = client.query_opt_cached(
            || "SELECT user_id, room_id, editor_id, thread_id, updated_at, deleted_at, content, pinned FROM lantern.messages WHERE id = $1",
            &[&id]
        ).await?;

        Ok(match row {
            None => None,
            Some(row) => Some(Message {
                id,
                user_id: row.try_get(0)?,
                room_id: row.try_get(1)?,
                editor_id: row.try_get(2)?,
                thread_id: row.try_get(3)?,
                updated_at: row.try_get(4)?,
                deleted_at: row.try_get(5)?,
                content: row.try_get(6)?,
                pinned: row.try_get(7)?,
            }),
        })
    }

    pub async fn upsert(&self, client: &Client) -> Result<(), ClientError> {
        let _ = client
            .execute_cached(
                || "CALL lantern.upsert_msg($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[
                    &self.id,
                    &self.user_id,
                    &self.room_id,
                    &self.thread_id,
                    &self.editor_id,
                    &self.updated_at,
                    &self.deleted_at,
                    &self.content,
                    &self.pinned,
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn search(
        client: &Client,
        room_id: Snowflake,
        limit: u8,
        mode: MessageSearch,
    ) -> Result<impl Stream<Item = Result<Message, ClientError>>, ClientError> {
        let limit = limit as i16;

        let stream = match mode {
            MessageSearch::After(ts) => {
                client
                    .query_stream_cached(
                        || "SELECT id, user_id, thread_id, editor_id, updated_at, deleted_at, content, pinned FROM lantern.messages WHERE room_id = $1 AND id > $2 LIMIT $3",
                        &[&room_id, &ts, &limit],
                    )
                    .await?
                    .boxed()
            }
            MessageSearch::Before(ts) => {
                client
                    .query_stream_cached(
                        || "SELECT id, user_id, thread_id, editor_id, updated_at, deleted_at, content, pinned FROM lantern.messages WHERE room_id = $1 AND id < $2 LIMIT $3",
                        &[&room_id, &ts, &limit],
                    )
                    .await?
                    .boxed()
            }
            _ => unimplemented!(),
        };

        Ok(stream.map(move |res| match res {
            Err(e) => Err(e),
            Ok(row) => Ok(Message {
                id: row.try_get(0)?,
                user_id: row.try_get(1)?,
                room_id,
                editor_id: row.try_get(2)?,
                thread_id: row.try_get(3)?,
                updated_at: row.try_get(4)?,
                deleted_at: row.try_get(5)?,
                content: row.try_get(6)?,
                pinned: row.try_get(7)?,
            }),
        }))
    }

    /*
    pub async fn get_attachments(
        &self,
        client: &Client,
    ) -> Result<impl Iterator<Item = Attachment>, ClientError> {
        let rows = client
            .query_cached(
                || "SELECT * FROM attachment WHERE message_id = $1",
                &[&self.id],
            )
            .await?;

        Ok(rows.into_iter().map(|row| Attachment::from_row(&row)))
    }

    pub async fn get_thread(&self, client: &Client) -> Result<Option<Thread>, ClientError> {
        client
            .query_opt_cached(|| "SELECT * FROM thread WHERE id = $1", &[&self.thread_id])
            .await
            .map(|row| row.as_ref().map(Thread::from_row))
    }
     */
}
