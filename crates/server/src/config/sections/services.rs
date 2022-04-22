#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Services {
    pub hcaptcha_secret: String,
    pub hcaptcha_sitekey: String,
}
