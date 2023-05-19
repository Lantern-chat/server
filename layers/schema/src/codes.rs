/// THIS MUST MATCH `lantern.to_language` in the database
///
/// Retreived from PostgreSQL via `SELECT * FROM pg_ts_config`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LanguageCode {
    English = 0,
    Simple = 1,
    Arabic = 2,
    Armenian = 3,
    Basque = 4,
    Catalan = 5,
    Danish = 6,
    Dutch = 7,
    Finnish = 8,
    French = 9,
    German = 10,
    Greek = 11,
    Hindi = 12,
    Hungarian = 13,
    Indonesian = 14,
    Irish = 15,
    Italian = 16,
    Lithuanian = 17,
    Nepali = 18,
    Norwegian = 19,
    Portuguese = 20,
    Romanian = 21,
    Russian = 22,
    Serbian = 23,
    Spanish = 24,
    Swedish = 25,
    Tamil = 26,
    Turkish = 27,
    Yiddish = 28,
}
