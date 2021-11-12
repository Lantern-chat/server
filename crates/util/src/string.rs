use smol_str::SmolStr;

pub enum SmolCowStr {
    Reused(SmolStr),
    Owned(String),
}
