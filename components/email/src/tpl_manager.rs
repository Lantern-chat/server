use ramhorns::{Error, Ramhorns};
use std::path::PathBuf;

pub struct TemplateManager {
    inner: tokio::sync::RwLock<Ramhorns<foldhash::fast::RandomState>>,
    dir: PathBuf,
    ext: &'static str,
}

impl TemplateManager {
    /// Create a new `TemplateManager` from a directory and file extension, eagerly loading all templates.
    pub async fn new(dir: impl Into<PathBuf>, ext: &'static str) -> Result<Self, Error> {
        let dir = dir.into();

        Ok(Self {
            inner: {
                let dir = dir.clone();

                let tpls = tokio::task::spawn_blocking(move || Ramhorns::from_folder_with_extension(&dir, ext))
                    .await
                    .expect("Unable to spawn task")?;

                tpls.into()
            },
            dir,
            ext,
        })
    }

    /// Asynchronously reload all templates from the directory.
    pub async fn reload(&self) -> Result<(), Error> {
        let (dir, ext) = (self.dir.clone(), self.ext);

        let new_tpls = tokio::task::spawn_blocking(move || Ramhorns::from_folder_with_extension(dir, ext))
            .await
            .expect("Unable to spawn task");

        *self.inner.write().await = new_tpls?;

        Ok(())
    }

    pub async fn render(&self, email: &crate::Email) -> Option<String> {
        let tpls = self.inner.read().await;
        let out = tpls.get(email.scenario.path())?.render(email);

        Some(out)
    }
}
