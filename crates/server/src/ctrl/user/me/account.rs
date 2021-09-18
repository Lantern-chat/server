use smol_str::SmolStr;

#[derive(Deserialize)]
pub struct ModifyAccountForm {
    pub current_password: SmolStr,

    #[serde(default)]
    pub new_password: Option<SmolStr>,
    #[serde(default)]
    pub new_username: Option<SmolStr>,
    #[serde(default)]
    pub new_email: Option<SmolStr>,
}
