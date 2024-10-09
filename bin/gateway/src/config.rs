pub mod sections {
    use schema::auth::BotTokenKey;
    use std::{net::SocketAddr, path::PathBuf};

    config::section! {
        #[serde(default)]
        pub struct Paths {
            /// Path to SSL Certificates
            ///
            /// Defualts to the current directory
            pub cert_path: PathBuf = PathBuf::default() => "CERT_PATH",

            /// Path to SSL Key file
            ///
            /// Defualts to the current directory
            pub key_path: PathBuf = PathBuf::default() => "KEY_PATH",

            /// Path to static frontend files (typically `./frontend`)
            pub web_path: PathBuf = "./frontend".into() => "WEB_PATH",

            /// Where to write logfiles to. Automatically rotated.
            pub log_dir: PathBuf = "./logs".into() => "LANTERN_LOG_DIR",
        }
    }

    config::section! {
        pub struct Web {
            /// Bind address
            pub bind: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 8080)) => "LANTERN_BIND" | config::util::parse_address,
        }
    }

    config::section! {
        pub struct Keys {
            /// Bot Token Key (padded)
            ///
            /// Used for signing bot tokens
            #[serde(with = "config::util::hex_key::loose")]
            pub bt_key: BotTokenKey = util::rng::gen_crypto_bytes().into() => "BT_KEY" | config::util::parse_hex_key[false],
        }
    }
}

config::config! {
    pub struct LocalConfig {
        /// Overall server configuration
        general: config::general::General,

        /// Filesystem paths
        paths: sections::Paths,

        /// Cryptographic keys
        keys: sections::Keys,

        /// Web/HTTP Configuration
        web: sections::Web,
    }
}

pub struct Config {
    pub local: LocalConfig,
    pub shared: schema::config::SharedConfig,
}
