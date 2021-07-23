use std::fs::OpenOptions;
use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};

use schema::Snowflake;

use tokio::fs::{self, File as TkFile};
use tokio::io::AsyncSeekExt;

use crate::filesystem::path::{id_to_name, id_to_path};

pub struct Disk {
    pub root: PathBuf,
}

impl Disk {
    pub fn new<P: AsRef<Path>>(root: P) -> Disk {
        Disk {
            root: root.as_ref().to_owned(),
        }
    }

    pub async fn open(&self, id: Snowflake, options: OpenOptions) -> io::Result<TkFile> {
        let mut path = self.root.clone();
        id_to_path(id, &mut path);
        id_to_name(id, &mut path);

        tokio::fs::OpenOptions::from(options).open(path).await
    }
}
