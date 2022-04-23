use std::path::Path;

pub mod sections {
    use super::util;

    macro_rules! section {
        (
            $(#[$meta:meta])*
            $vis:vis struct $name:ident {$(
                $(#[$field_meta:meta])*
                $field_vis:vis $field_name:ident : $field_ty:ty = $field_default:expr $(=> $field_env:literal $(| $func:ident)?)?
            ),*$(,)?}
        ) => {
            $(#[$meta])*
            $vis struct $name {$(
                $(#[$field_meta])*
                $field_vis $field_name: $field_ty,
            )*}

            impl Default for $name {
                fn default() -> Self {
                    $name {$(
                        $field_name: $field_default,
                    )*}
                }
            }

            impl $name {
                pub fn apply_overrides(&mut self) {$($(
                    if let Ok(value) = std::env::var($field_env) {
                        log::debug!("Applying environment overwrite for {}.{}=>{}", stringify!($name), stringify!($field_name), $field_env);
                        self.$field_name = ($($func(&value),)? value , ).0.into();
                    }
                )?)*}
            }
        };
    }

    pub mod account;
    pub mod db;
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

macro_rules! decl_config {
    ($(
        $(#[$meta:meta])*
        $field:ident: $field_ty:ty
    ),*$(,)?) => {

        #[derive(Default, Debug, Serialize, Deserialize)]
        #[serde(default)]
        pub struct Config {$(
            $(#[$meta])*
            pub $field: $field_ty,
        )*}

        impl Config {
            pub fn apply_overrides(&mut self) {
                $(self.$field.apply_overrides();)*
            }
        }
    };
}

decl_config! {
    general: sections::general::General,
    paths: sections::paths::Paths,
    db: sections::db::Database,
    account: sections::account::Account,
    message: sections::message::Message,
    party: sections::party::Party,
    upload: sections::upload::Upload,
    services: sections::services::Services,
    keys: sections::keys::Keys,
    tasks: sections::tasks::Tasks,
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

pub async fn load(path: impl AsRef<Path>) -> io::Result<(bool, Config)> {
    let path = path.as_ref();

    let file = match tokio::fs::read_to_string(path).await {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            log::warn!("{} not found, generating default config", path.display());

            return Ok((true, Config::default()));
        }
        Err(e) => return Err(e),
    };

    let parsed = match get_format(path) {
        Format::TOML => toml::from_str(&file).map_err(|e| io::Error::new(ErrorKind::InvalidData, e)),
        Format::JSON => serde_json::from_str(&file).map_err(|e| io::Error::new(ErrorKind::InvalidData, e)),
    };

    parsed.map(|config| (false, config))
}

pub async fn save(path: impl AsRef<Path>, config: &Config) -> io::Result<()> {
    let path = path.as_ref();

    let file = match get_format(path) {
        Format::TOML => toml::to_string(config).map_err(|e| io::Error::new(ErrorKind::InvalidData, e)),
        Format::JSON => serde_json::to_string(config).map_err(|e| io::Error::new(ErrorKind::InvalidData, e)),
    }?;

    tokio::fs::write(path, file).await
}
