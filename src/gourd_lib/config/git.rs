use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use git2::Repository;
use log::debug;
use log::warn;

use crate::bailc;
use crate::config::GitResource;
use crate::ctx;
use crate::file_system::FileOperations;
use crate::resources::run_script;
use crate::constants::CMD_STYLE;

/// Opens a named submodule of the current directory as a Git repo.
fn open_submodule(submodule_name: &str) -> Result<Repository> {
    let parent_repo = Repository::open_from_env()
        .with_context(ctx!("Could not open the parent Git repository.", ;
                    "To specify submodules, the current directory must be a repository.", ))?;


    let mut submodule = parent_repo.find_submodule(submodule_name)
        .with_context(ctx!("Could not open the Git submodule '{}'.", submodule_name;
                            "Refer to the manual for correct submodule configuration.", ))?;
    debug!(
        "Found the '{}' submodule. Attempting to open it.",
        submodule_name
    );

    // submodule.update(false, None).with_context(
    //     ctx!("Could not update the Git submodule '{}'.", submodule_name;
    //             "Refer to the manual for correct submodule configuration.", ),
    // )?;

    Ok(submodule.open().with_context(
        ctx!("Could not open the Git submodule '{}'.", submodule_name;
                "Try to run {CMD_STYLE}git submodule update --init . {CMD_STYLE:#}.", ),
    )?)
}

/// Checks out the requested revision of the Git repo.
fn checkout_rev(repo: &Repository, rev: &str, enable_fetch: bool) -> Result<()> {
    debug!("Checking out '{}' using Git.", rev);

    let mut parse_attempt = repo.revparse_single(rev);

    if enable_fetch && parse_attempt.is_err() {
        warn!(
            "Could not parse the reference '{}'. Trying to fetch from 'origin'.",
            rev
        );
        let mut remote = repo
            .find_remote("origin")
            .with_context(ctx!("The Git submodule does not have a remote 'origin'.", ;
                               "You can disable 'Git fetch' in the configuration.", ))?;

        remote
            .fetch(&[rev], None, None)
            .with_context(ctx!("Cannot fetch '{}' from 'origin'.", rev;
                            "Check your remote configuration and commit/tag.", ))?;

        parse_attempt = repo.revparse_single(rev);
    }

    let object = parse_attempt
        .with_context(ctx!("Could not find the '{}' revision spec.", &rev;
            "Change this to a valid commit or tag.", ))?;

    repo.checkout_tree(&object, None)
        .with_context(ctx!("Could not perform a Git checkout to '{}'.", rev;
                "Is your working tree clean?", ))?;

    repo.set_head_detached(object.id())
        .with_context(ctx!("Could not set the HEAD to '{}'.", rev;
                "Check that the object ID {} is valid.", object.id()))?;

    Ok(())
}

impl GitResource {
    /// Open the linked Git repository, which should already be
    /// defined as a Git submodule, check out the requested revision,
    /// and copy a file.
    ///
    /// If successful, returns a path to the copy.
    pub fn fetch(&self, fs: &impl FileOperations) -> Result<PathBuf> {
        if !self.store.exists() {
            debug!("Cached file not present at {:?}. Using Git.", self.store);

            let submodule_repo = open_submodule(&self.submodule_name)?;

            match &self.rev {
                Some(rev_spec) => checkout_rev(&submodule_repo, rev_spec, self.enable_fetch)?,
                None => {}
            };

            // Unsure if it is possible to have a bare submodule, including the message just
            // in case.
            let submodule_dir = submodule_repo.workdir().with_context(
                ctx!("Cannot retrieve files from the bare repository '{}'", &self.submodule_name;
                     "Ensure that the submodule is not bare.", ),
            )?;

            match &self.compile_script {
                Some(script) => {
                    let script_file = fs.canonicalize(&submodule_dir.join(script))
                    .with_context(ctx!("The compile script {:?} cannot be accessed.", script;
                                       "Verify that the path is correct (relative to the submodule).", ))?;

                    let exit_code = run_script(script_file, vec![], submodule_dir)
                        .with_context(ctx!("Could not run the compile script '{:?}'.", script;
                                 "Verify that the script is executable.", ))?;

                    if !exit_code.success() {
                        bailc!("The compile script '{:?}' failed.", script;
                               "The exit code was not zero: {}", exit_code;
                               "Verify that the script runs correctly with '{:?}' as the working directory.",
                               submodule_dir);
                    }
                }
                None => {}
            }

            let file_path = fs.canonicalize(&submodule_dir.join(&self.file))
                .with_context(ctx!("Could not get {:?} from the Git submodule.", &self.file;
                                   "Ensure that the file exists or is created by a compile script.", ))?;

            let copied_path = fs.copy(&file_path, &self.store)?;

            Ok(copied_path)
        } else {
            warn!(
                "A file exists at {:?}. Not updating it from Git.",
                self.store
            );

            Ok(self.store.clone())
        }
    }
}
