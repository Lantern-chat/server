use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
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
