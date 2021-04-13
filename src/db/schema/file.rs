use std::str::FromStr;

use super::{Client, ClientError, Snowflake};

pub use serde_shims::mime::Wrapper as Mime;

bitflags::bitflags! {
    pub struct FileFlags: i16 {
        const INCOMPLETE = 1;
    }
}

serde_shims::bitflags::impl_serde_for_bitflags!(FileFlags);

#[derive(Serialize)]
pub struct File {
    pub id: Snowflake,
    pub name: String,
    pub preview: Option<Vec<u8>>,
    pub mime: Option<Mime>,
    pub size: u32,
    pub offset: u32,

    /// SHA3-256 hash
    pub sha3: Option<Box<[u8]>>,

    pub flags: FileFlags,
}

impl File {
    pub async fn upsert(&self, client: &Client) -> Result<(), ClientError> {
        let mime: Option<&str> = self.mime.as_ref().map(|mime| mime.as_ref());
        let flags = self.flags.bits();

        client
            .write
            .execute_cached(
                || "CALL lantern.upsert_file($1, $2, $3, $4, $5, $6, $7, $8)",
                &[
                    &self.id,
                    &self.name,
                    &self.preview,
                    &mime,
                    &self.size,
                    &self.offset,
                    &flags,
                    &self.sha3.as_ref().map(|sha3| &sha3[..]),
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn find(id: Snowflake, client: &Client) -> Result<Option<File>, ClientError> {
        let row = client.read
            .query_opt_cached(
                || "SELECT name, preview, mime, size, \"offset\", sha3, flags FROM lantern.files WHERE id = $1",
                &[&id],
            )
            .await?;

        match row {
            None => Ok(None),
            Some(row) => Ok(Some({
                let mime: Option<String> = row.try_get(2)?;
                let sha3: Option<Vec<u8>> = row.try_get(5)?;

                File {
                    id,
                    name: row.try_get(0)?,
                    preview: row.try_get(1)?,
                    mime: mime.and_then(|mime| Mime::from_str(&mime).ok()),
                    size: row.try_get::<_, i32>(3)? as u32,
                    offset: row.try_get::<_, i32>(4)? as u32,
                    flags: FileFlags::from_bits_truncate(row.try_get(6)?),
                    sha3: sha3.map(|sha3| sha3.into_boxed_slice()),
                }
            })),
        }
    }

    pub async fn update_offset(&self, client: &Client) -> Result<(), ClientError> {
        let offset = self.offset as i32;

        client
            .write
            .execute_cached(
                || "UPDATE lantern.files SET \"offset\" = $2 WHERE id = $1",
                &[&self.id, &offset],
            )
            .await?;

        Ok(())
    }
}
