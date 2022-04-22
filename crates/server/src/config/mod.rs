use std::path::Path;

pub mod sections {
    use super::util;

    pub mod account;
    pub mod general;
    pub mod keys;
    pub mod message;
    pub mod party;
    pub mod paths;
    pub mod services;
    pub mod tasks;
    pub mod upload;
}

pub mod util;

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: sections::general::General,
    pub paths: sections::paths::Paths,
    pub account: sections::account::Account,
    pub message: sections::message::Message,
    pub party: sections::party::Party,
    pub upload: sections::upload::Upload,
    pub services: sections::services::Services,
    pub keys: sections::keys::Keys,
    pub tasks: sections::tasks::Tasks,
}

enum Format {
    TOML,
    JSON,
}

use std::io::{self, ErrorKind};

fn get_format(path: &Path) -> Format {
    let mut format = Format::TOML;
    if let Some(ext) = path.extension() {
        if ext.eq_ignore_ascii_case("toml") {
            format = Format::TOML;
        } else if ext.eq_ignore_ascii_case("json") {
            format = Format::JSON;
        }
    }
    format
}

pub async fn load(path: impl AsRef<Path>) -> io::Result<Config> {
    let path = path.as_ref();

    let file = match tokio::fs::read_to_string(path).await {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            log::warn!("{} not found, generating default config", path.display());

            return Ok(Config::default());
        }
        Err(e) => return Err(e),
    };

    match get_format(path) {
        Format::TOML => toml::from_str(&file).map_err(|e| io::Error::new(ErrorKind::InvalidData, e)),
        Format::JSON => serde_json::from_str(&file).map_err(|e| io::Error::new(ErrorKind::InvalidData, e)),
    }
}

pub async fn save(path: impl AsRef<Path>, config: &Config) -> io::Result<()> {
    let path = path.as_ref();

    let file = match get_format(path) {
        Format::TOML => toml::to_string(config).map_err(|e| io::Error::new(ErrorKind::InvalidData, e)),
        Format::JSON => serde_json::to_string(config).map_err(|e| io::Error::new(ErrorKind::InvalidData, e)),
    }?;

    tokio::fs::write(path, file).await
}
