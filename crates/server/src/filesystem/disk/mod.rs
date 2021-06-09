use std::io;
use std::path::{Path, PathBuf};

use db::Snowflake;

use io::SeekFrom;
use tokio::fs::{self, File};
use tokio::io::AsyncSeekExt;

pub mod path;

pub struct FileStore {
    pub root: PathBuf,
}

impl FileStore {
    pub fn new<P: AsRef<Path>>(root: P) -> FileStore {
        FileStore {
            root: root.as_ref().to_owned(),
        }
    }

    pub async fn open(&self, id: Snowflake, offset: u64, read: bool) -> Result<File, io::Error> {
        let mut path = self.root.clone();

        // create directory structure
        path::id_to_path(&mut path, id);
        fs::create_dir_all(&path).await?;

        // append filename
        path::id_to_name(id, &mut path);

        let mut options = fs::OpenOptions::new();

        let mut file = options.read(read).write(!read).create(!read).open(path).await?;

        if offset != 0 {
            file.seek(SeekFrom::Start(offset)).await?;
        }

        Ok(file)
    }
}
