use std::path::PathBuf;

section! {
    #[serde(default)]
    pub struct Database {
        pub db_str: String = "postgresql://postgres:password@localhost:5432".to_owned() => "DB_STR",
        pub migrations: PathBuf = "./sql/migrations".into() => "MIGRATIONS",
    }
}
