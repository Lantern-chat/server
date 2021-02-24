use std::path::PathBuf;
use std::{fs, io};

use futures::{future, FutureExt};

pub mod path;

pub struct DiskStore {
    root: PathBuf,
}

use io::SeekFrom;
use tokio::{fs::File as TkFile, io::AsyncSeekExt, task::JoinHandle};

use crate::db::Snowflake;

use super::{File, FileStore};

pub struct DiskFile {
    file: TkFile,
}

impl File<DiskStore> for DiskFile {
    type CloseFileFuture = GenericIOFuture<()>;
    fn close(mut self) -> Self::CloseFileFuture {
        GenericIOFuture {
            handle: tokio::task::spawn(async move {
                self.file.sync_all().await;
                let file = self.file.into_std().await;

                drop(file);

                Ok(())
            }),
        }
    }

    type SeekFileFuture = GenericIOFuture<u64>;
    fn seek(&mut self, seek: SeekFrom) -> Self::SeekFileFuture {
        GenericIOFuture {
            handle: tokio::task::spawn(self.file.seek(seek)),
        }
    }
}

impl FileStore for DiskStore {
    type Error = io::Error;
    type File = DiskFile;

    type InitializeFuture = future::Ready<Result<(), Self::Error>>;
    fn initialize(&self) -> Self::InitializeFuture {
        future::ok(())
    }

    type OpenFileFuture = GenericIOFuture<DiskFile>;
    fn open_file(&self, id: Snowflake, opt: fs::OpenOptions) -> Self::OpenFileFuture {
        GenericIOFuture {
            handle: tokio::task::spawn_blocking(move || -> Result<DiskFile, io::Error> {
                let path = path::id_to_path(id);

                fs::create_dir_all(path.parent().unwrap());

                let file = TkFile::from_std(opt.open(path)?);

                Ok(DiskFile { file })
            }),
        }
    }
}

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct GenericIOFuture<T> {
    handle: JoinHandle<Result<T, io::Error>>,
}

impl<T> Future for GenericIOFuture<T> {
    type Output = Result<T, std::io::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match futures::ready!(self.handle.poll_unpin(cx)) {
            Ok(res) => Poll::Ready(res),
            Err(e) => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::Other,
                "background task failed",
            ))),
        }
    }
}
