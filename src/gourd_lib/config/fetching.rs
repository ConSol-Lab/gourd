use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use git2::build::RepoBuilder;
use log::debug;
use log::warn;

use super::GitProgram;
use crate::bailc;
use crate::config::FetchedResource;
use crate::ctx;
use crate::file_system::FileOperations;
use crate::resources::run_script;

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

/// Fetch a program from a git repository.
pub fn fetch_git(program: &GitProgram) -> Result<PathBuf> {
    debug!("Fetching git program from {}", program.git_uri);

    let repo_base = PathBuf::from(format!("./{}", program.commit_id));

    if repo_base.exists() {
        debug!("Not cloning {} becuase it exists", program.git_uri)
    }

    let repo = RepoBuilder::new()
        .clone(&program.git_uri, &repo_base)
        .with_context(ctx!(
          "Could not clone to repository from {}", program.git_uri;
          "Make sure that the repository exists and you have access to it",
        ))?;

    let (object, reference) = repo
        .revparse_ext(&program.commit_id)
        .with_context(ctx!("Commit id {} is invalid", program.commit_id; "",))?;

    match reference {
        Some(gref) => repo.set_head(gref.name().unwrap()),
        None => repo.set_head_detached(object.id()),
    }
    .with_context(ctx!("Could not move the git head to {}", program.commit_id; "",))?;

    let bc = program.build_command.clone();

    debug!("Running build command {}", bc);

    let augumented = vec!["-c", &bc];

    run_script("sh", augumented, &repo_base)?;

    Ok(repo_base.join(program.path.clone()))
}
