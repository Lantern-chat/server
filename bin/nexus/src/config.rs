pub mod sections {
    use aes::{cipher::Key, Aes128, Aes256};
    use schema::auth::BotTokenKey;
    use std::{net::SocketAddr, path::PathBuf};
    use uuid::Uuid;

    config::section! {
        #[serde(default)]
        pub struct Node {
            /// Node UUID if this is a faction node, otherwise indicates a user nexus node.
            pub faction: Uuid = Uuid::nil() => "LANTERN_FACTION_UUID" | config::util::parse_uuid,
        }
    }

    impl Node {
        pub fn is_faction(&self) -> bool {
            !self.faction.is_nil()
        }

        pub fn is_user_nexus(&self) -> bool {
            self.faction.is_nil()
        }

        pub fn faction_id(&self) -> Option<Uuid> {
            if self.is_user_nexus() {
                None
            } else {
                Some(self.faction)
            }
        }
    }

    config::section! {
        #[serde(default)]
        pub struct Paths {
            /// Path to SSL Certificates
            ///
            /// Defualts to the current directory
            pub cert_path: PathBuf = PathBuf::default() => "LANTERN_RPC_CERT_PATH",

            /// Path to SSL Key file
            ///
            /// Defualts to the current directory
            pub key_path: PathBuf = PathBuf::default() => "LANTERN_RPC_KEY_PATH",

            /// Where to write logfiles to. Automatically rotated.
            pub log_dir: PathBuf = "./logs".into() => "LANTERN_RPC_LOG_DIR",
        }
    }

    config::section! {
        pub struct Database {
            /// Database connection string, can also be a unix socket.
            pub db_str: String = "postgresql://postgres:password@localhost:5432/lantern".to_owned() => "LANTERN_RPC_DB_STR",
        }
    }

    config::section! {
        pub struct Rpc {
            /// Bind address
            pub bind: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 8080)) => "LANTERN_RPC_BIND" | config::util::parse_address,
        }
    }

    config::section! {
        pub struct Keys {
            /// Multi-factor authentication encryption key
            #[serde(with = "config::util::hex_key")]
            pub mfa_key: Key<Aes256> = util::rng::crypto_thread_rng().gen_bytes().into() => "MFA_KEY" | config::util::parse_hex_key[true],

            /// Some snowflakes are encrypted as a form of reversable obfuscation.
            #[serde(with = "config::util::hex_key")]
            pub sf_key: Key<Aes128> = util::rng::crypto_thread_rng().gen_bytes().into() => "SF_KEY" | config::util::parse_hex_key[true],

            /// Bot Token Key (padded)
            ///
            /// Used for signing bot tokens
            #[serde(with = "config::util::hex_key::loose")]
            pub bt_key: BotTokenKey = util::rng::crypto_thread_rng().gen_bytes().into() => "BT_KEY" | config::util::parse_hex_key[false],
        }
    }
}

config::config! {
    pub struct LocalConfig {
        /// Node configuration
        node: sections::Node,
        /// Overall server configuration
        general: config::general::General,
        /// Filesystem paths
        paths: sections::Paths,
        /// Database configuration
        db: sections::Database,
        /// Cryptographic keys
        keys: sections::Keys,
        /// RPC Configuration
        rpc: sections::Rpc,
    }
}

pub struct Config {
    pub shared: schema::config::SharedConfig,
    pub local: LocalConfig,
}
