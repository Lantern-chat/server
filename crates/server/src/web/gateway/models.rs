use schema::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    id: Snowflake,
    username: String,
    discriminator: String,
    email: Option<String>,
}
