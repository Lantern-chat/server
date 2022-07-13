use std::path::PathBuf;

section! {
    #[serde(default)]
    pub struct Paths {
        /// Path to where user data will be stored
        pub data_path: PathBuf = PathBuf::default() => "DATA_DIR",
        /// Path to SSL Certificates
        pub cert_path: PathBuf = PathBuf::default() => "CERT_PATH",
        /// Path to SSL Key file
        pub key_path: PathBuf = PathBuf::default() => "KEY_PATH",

        /// Path to static frontend files (typically `./frontend`)
        pub web_path: PathBuf = PathBuf::from("./frontend") => "WEB_PATH",
    }
}
