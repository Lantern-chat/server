pub mod sections {
    use schema::auth::BotTokenKey;
    use std::{net::SocketAddr, path::PathBuf};

    config::section! {
        #[serde(default)]
        pub struct Paths {
            /// Where to write logfiles to. Automatically rotated.
            pub log_dir: PathBuf = "./logs".into() => "LANTERN_LOG_DIR",
        }
    }

    config::section! {
        pub struct Web {
            /// Bind address
            pub bind: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 8080)) => "LANTERN_BIND" | config::util::parse_address,

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
        }
    }

    config::section! {
        pub struct Keys {
            /// Bot Token Key (padded)
            ///
            /// Used for signing bot tokens
            #[serde(with = "config::util::hex_key::loose")]
            pub bt_key: BotTokenKey = util::rng::crypto_thread_rng().gen_bytes().into() => "BT_KEY" | config::util::parse_hex_key[false],
        }
    }

    config::section! {
        pub struct Rpc {
            /// RPC Client certification path
            pub cert_path: PathBuf = PathBuf::default() => "RPC_CERT_PATH",

            /// RPC Nexus address, from which all other RPC endpoints will be fetched
            pub nexus_addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 8083)) => "NEXUS_ADDR" | config::util::parse_address,

            /// Maximum number of concurrent RPC connections per endpoint
            ///
            /// Defaults to 10
            pub max_conns: usize = 10 => "MAX_RPC_CONNS" | config::util::parse[10usize],
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

        /// RPC Configuration
        rpc: sections::Rpc,
    }
}

pub use schema::config::SharedConfig;

pub struct Config {
    pub local: LocalConfig,
    pub shared: SharedConfig,
}
