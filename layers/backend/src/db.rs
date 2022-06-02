use db::pool::Pool;

#[derive(Clone)]
pub struct DatabasePools {
    pub read: Pool,
    pub write: Pool,
}
