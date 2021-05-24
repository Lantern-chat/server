use db::pool::Pool;

#[derive(Clone)]
pub struct DatabasePools {
    read: Pool,
    write: Pool,
}