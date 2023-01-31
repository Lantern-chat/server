use std::path::PathBuf;

section! {
    #[serde(default)]
    pub struct Database {
        /// Database connection string
        pub db_str: String = "postgresql://postgres:password@localhost:5432/lantern".to_owned() => "DB_STR",

        /// Path to database migration scripts
        pub migrations: PathBuf = "./sql/migrations".into() => "MIGRATIONS",
    }
}
