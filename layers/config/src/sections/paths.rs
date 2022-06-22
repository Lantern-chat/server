use std::path::PathBuf;

section! {
    #[serde(default)]
    pub struct Paths {
        pub data_path: PathBuf = PathBuf::default() => "DATA_DIR",
        pub cert_path: PathBuf = PathBuf::default() => "CERT_PATH",
        pub key_path: PathBuf = PathBuf::default() => "KEY_PATH",
    }
}
