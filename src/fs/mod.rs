
pub mod disk;

pub trait FileStore {
    type Error;

    fn initialize(&self) -> Result<(), Self::Error>;
}
