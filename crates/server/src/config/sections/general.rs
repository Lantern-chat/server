#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub server_name: String,
}

impl Default for General {
    fn default() -> General {
        General {
            server_name: "Lantern Chat".to_owned(),
        }
    }
}
