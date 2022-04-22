use std::{env, path::PathBuf};

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Paths {
    pub data_path: PathBuf,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}
