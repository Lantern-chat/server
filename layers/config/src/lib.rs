#[macro_use]
extern crate serde;
extern crate tracing as log;

use std::path::Path;

const KIBIBYTE: i32 = 1024;
const MIBIBYTE: i32 = KIBIBYTE * 1024;
const GIBIBYTE: i64 = MIBIBYTE as i64 * 1024;

pub mod sections {
    use super::util;

    macro_rules! section {
        (
            $(#[$meta:meta])*
            $vis:vis struct $name:ident {$(
                $(#[$field_meta:meta])*
                $field_vis:vis $field_name:ident : $field_ty:ty = $field_default:expr
                    $(=> $field_env:literal
                        $(| $func:path
                            $([  $($param:expr),* ])?
                        )?
                    )?
            ),*$(,)?}
        ) => {
            #[derive(Debug, Serialize, Deserialize)]
            $(#[$meta])*
            #[serde(deny_unknown_fields)]
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
                        self.$field_name = ($($func(&value $( $(,$param)* )? ),)? value , ).0.into();
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
    pub mod web;
}

pub mod util;

macro_rules! decl_config {
    ($(
        $(#[$meta:meta])*
        $field:ident: $field_ty:ty
    ),*$(,)?) => {

        #[derive(Default, Debug, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        #[cfg_attr(not(feature = "strict"), serde(default))]
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
    web: sections::web::Web,
}

enum Format {
    TOML,
    JSON,
}

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

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("TOML Parse Error: {0}")]
    TomlDeError(#[from] toml::de::Error),
    #[error("TOML Format Error: {0}")]
    TomlSeError(#[from] toml::ser::Error),

    #[error("JSON Error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl Config {
    pub async fn load(path: impl AsRef<Path>) -> Result<Config, ConfigError> {
        let path = path.as_ref();

        let file = tokio::fs::read_to_string(path).await?;

        Ok(match get_format(path) {
            Format::TOML => toml::from_str(&file)?,
            Format::JSON => serde_json::from_str(&file)?,
        })
    }

    pub fn configure(&mut self) {
        self.general.configure();
        self.web.configure();
    }

    pub async fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();

        let file = match get_format(path) {
            Format::TOML => toml::to_string(self)?,
            Format::JSON => serde_json::to_string(self)?,
        };

        tokio::fs::write(path, file).await?;

        Ok(())
    }
}
