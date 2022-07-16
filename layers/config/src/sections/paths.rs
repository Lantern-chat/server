use std::path::PathBuf;

section! {
    #[serde(default)]
    pub struct Paths {
        /// Path to where user data will be stored
        ///
        /// Defualts to the current directory
        pub data_path: PathBuf = PathBuf::default() => "DATA_DIR",

        /// Path to SSL Certificates
        ///
        /// Defualts to the current directory
        pub cert_path: PathBuf = PathBuf::default() => "CERT_PATH",

        /// Path to SSL Key file
        ///
        /// Defualts to the current directory
        pub key_path: PathBuf = PathBuf::default() => "KEY_PATH",

        /// Path to static frontend files (typically `./frontend`)
        pub web_path: PathBuf = PathBuf::from("./frontend") => "WEB_PATH",

        /// Path to compiled utility binaries (defaults to `./server/target/release`)
        pub bin_path: PathBuf = PathBuf::from("./server/target/release") => "BIN_PATH",
    }
}
