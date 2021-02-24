pub mod disk;

/*
use futures::Future;

use crate::db::Snowflake;

use std::io::SeekFrom;

pub trait File<FS: FileStore>: Sized {
    type CloseFileFuture: Future<Output = Result<(), FS::Error>>;
    fn close(self) -> Self::CloseFileFuture;

    type SeekFileFuture: Future<Output = Result<u64, FS::Error>>;
    fn seek(&mut self, seek: SeekFrom) -> Self::SeekFileFuture;
}

pub trait FileStore: Sized {
    type Error;
    type File: File<Self>;

    type InitializeFuture: Future<Output = Result<(), Self::Error>>;
    fn initialize(&self) -> Self::InitializeFuture;

    type OpenFileFuture: Future<Output = Result<Self::File, Self::Error>>;
    fn open_file(&self, id: Snowflake, opt: std::fs::OpenOptions) -> Self::OpenFileFuture;
}
 */
