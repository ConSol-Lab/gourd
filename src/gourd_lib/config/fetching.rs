use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use log::warn;

use crate::bailc;
use crate::config::FetchedResource;
use crate::file_system::FileOperations;

impl<const PERM: u32> FetchedResource<PERM> {
    /// Fetch a remote resource and save it to a file.
    ///
    /// If successful, returns a path to the saved file
    pub fn fetch(&self, fs: &impl FileOperations) -> Result<PathBuf> {
        if cfg!(feature = "fetching") {
            #[cfg(feature = "fetching")]
            {
                use crate::network::download_file;
                if !self.store.exists() {
                    download_file(&self.url, &self.store, fs)?;
                    fs.set_permissions(&self.store, PERM)?;
                } else {
                    warn!(
                        "File {} already exists, won't download again",
                        self.store.display()
                    );
                }

                Ok(self.store.clone())
            }
        } else {
            bailc!(
                "Could not fetch remote resource",;
                "this version of gourd was built without fetching support",;
                "do not use urls",
            );
        }
    }
}
