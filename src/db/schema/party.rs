use futures::{Stream, StreamExt};

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    pub id: Snowflake,
    pub username: String,
    pub discriminator: i16,
    pub is_verified: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_id: Option<Snowflake>,
}

impl Party {
    pub async fn insert(&self, client: &Client) -> Result<(), ClientError> {
        client
            .execute_cached(
                || "INSERT INTO lantern.party (id, owner_id, name) VALUES ($1, $2, $3)",
                &[&self.id, &self.owner_id, &self.name],
            )
            .await?;

        Ok(())
    }

    pub async fn find(client: &Client, id: Snowflake) -> Result<Option<Self>, ClientError> {
        let row = client
            .query_opt_cached(
                || "SELECT owner_id, name FROM lantern.party WHERE id = $1",
                &[&id],
            )
            .await?;

        match row {
            None => Ok(None),
            Some(row) => Ok(Some(Party {
                id,
                owner_id: row.try_get(0)?,
                name: row.try_get(1)?,
            })),
        }
    }

    pub async fn get_members(
        &self,
        client: &Client,
    ) -> Result<impl Stream<Item = Result<PartyMember, ClientError>>, ClientError> {
        let stream = client.query_stream_cached(
            || "SELECT id, username, discriminator, is_verified, COALESCE(party_member.nickname, users.nickname), custom_status, biography, COALESCE(party_member.avatar_id, users.avatar_id) FROM users LEFT JOIN party_member ON id = user_id WHERE party_id = $1",
            &[&self.id],
        ).await?;

        Ok(stream.map(|res| match res {
            Err(e) => Err(e),
            Ok(row) => Ok(PartyMember {
                id: row.try_get(0)?,
                username: row.try_get(1)?,
                discriminator: row.try_get(2)?,
                is_verified: row.try_get(3)?,
                nickname: row.try_get(4)?,
                status: row.try_get(5)?,
                bio: row.try_get(6)?,
                avatar_id: row.try_get(7)?,
            }),
        }))
    }

    pub async fn has_member(
        &self,
        user_id: Snowflake,
        client: &Client,
    ) -> Result<bool, ClientError> {
        let count = client
            .execute_cached(
                || "SELECT FROM lantern.party_member WHERE party_id = $1 AND user_id = $2",
                &[&self.id, &user_id],
            )
            .await?;

        Ok(count > 0)
    }

    /*
    pub async fn get_roles(
        &self,
        client: &Client,
    ) -> Result<impl Iterator<Item = Role>, ClientError> {
        let rows = client
            .query_cached(|| "SELECT * FROM role WHERE party_id = $1", &[&self.id])
            .await?;

        Ok(rows.into_iter().map(|row| Role::from_row(&row)))
    }

    pub async fn get_owner(&self, client: &Client) -> Result<User, ClientError> {
        let row = client
            .query_one_cached(|| "SELECT * FROM user WHERE id = $1", &[&self.owner_id])
            .await?;

        Ok(User::from_row(&row))
    }
     */
}
