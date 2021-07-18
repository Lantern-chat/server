use std::io;
use std::path::{Path, PathBuf};

use schema::Snowflake;

use io::SeekFrom;
use tokio::fs::{self, File};
use tokio::io::AsyncSeekExt;

pub struct Disk {
    pub root: PathBuf,
}

impl Disk {
    pub fn new<P: AsRef<Path>>(root: P) -> Disk {
        Disk {
            root: root.as_ref().to_owned(),
        }
    }
}
