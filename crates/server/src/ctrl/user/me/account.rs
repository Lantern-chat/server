#[derive(Deserialize)]
pub struct ModifyAccountForm {
    pub current_password: String,

    #[serde(default)]
    pub new_password: Option<String>,
    #[serde(default)]
    pub new_username: Option<String>,
    #[serde(default)]
    pub new_email: Option<String>,
}
