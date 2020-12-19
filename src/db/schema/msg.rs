use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub session_id: Snowflake,
    pub user_id: Snowflake,
    pub editor_id: Option<Snowflake>,
    pub room_id: Snowflake,
    pub thread_id: Option<Snowflake>,
    pub updated_at: (),
    pub content: String,
    pub pinned: bool,
}

impl Message {
    pub async fn get_attachments(
        &self,
        client: &Client,
    ) -> Result<impl Iterator<Item = Attachment>, Error> {
        let rows = client
            .query_cached(CachedQuery::GetMessageAttachments, &[&self.id])
            .await?;

        Ok(rows.into_iter().map(|row| Attachment::from_row(&row)))
    }

    pub async fn get_thread(&self, client: &Client) -> Result<Option<Thread>, Error> {
        client
            .query_opt_cached(CachedQuery::GetMessageThread, &[&self.thread_id])
            .await
            .map(|row| row.as_ref().map(Thread::from_row))
    }
}
