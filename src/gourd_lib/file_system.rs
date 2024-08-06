use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use git2::Repository;
use log::debug;
use log::info;
use log::trace;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tar::Archive;

use crate::bailc;
use crate::error::ctx;

/// Interactor with the actual physical file system.
#[derive(Clone, Copy, Debug)]
pub struct FileSystemInteractor {
    /// If true this will not write nor store any state to the file system.
    pub dry_run: bool,
}

/// This defines all interactions of gourd with the filesystem.
pub trait FileOperations {
    /// Read a file into raw bytes.
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>>;

    /// Read a file into a utf8 string.
    fn read_utf8(&self, path: &Path) -> Result<String>;

    /// Try to deserialize a toml file into a struture `T`.
    fn try_read_toml<T: DeserializeOwned>(&self, path: &Path) -> Result<T>;

    /// Try to serialize a struct `T` into a toml file.
    fn try_write_toml<T: Serialize>(&self, path: &Path, data: &T) -> Result<()>;

    /// Write all files in a .tar directory structure at the provided path.
    fn write_archive<T: Read>(&self, path: &Path, data: Archive<T>) -> Result<()>;

    /// Write all bytes to a file.
    fn write_bytes_truncate(&self, path: &Path, bytes: &[u8]) -> Result<()>;

    /// Write a [String] to a file.
    fn write_utf8_truncate(&self, path: &Path, data: &str) -> Result<()>;

    /// Truncates the file and then runs [FileOperations::canonicalize].
    fn truncate_and_canonicalize(&self, path: &Path) -> Result<PathBuf>;

    /// Truncates the folder and then runs [FileOperations::canonicalize].
    fn truncate_and_canonicalize_folder(&self, path: &Path) -> Result<PathBuf>;

    /// Make a file possible to execute.
    fn set_permissions(&self, path: &Path, perms: u32) -> Result<()>;

    /// Given a path try to canonicalize it.
    ///
    /// This will fail for files that do not exist.
    fn canonicalize(&self, path: &Path) -> Result<PathBuf>;

    /// Create a new template repository.
    fn init_git_repository(&self, path: &Path) -> Result<()>;
}

impl FileOperations for FileSystemInteractor {
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        fs::read(path).with_context(ctx!(
          "Could not read the file {path:?}", ;
          "Ensure that the file exists and you have permissions to access it",
        ))
    }

    fn read_utf8(&self, path: &Path) -> Result<String> {
        String::from_utf8(self.read_bytes(path)?).with_context(ctx!(
          "{path:?} is not valid UTF-8", ;
          "The file doesn't seem to be human readable?",
        ))
    }

    fn try_read_toml<T: DeserializeOwned>(&self, path: &Path) -> Result<T> {
        toml::from_str::<T>(&self.read_utf8(path)?).with_context(ctx!(
          "Could not deserialize toml file {path:?}", ;
          "Ensure that the file is valid toml",
        ))
    }

    fn try_write_toml<T: Serialize>(&self, path: &Path, data: &T) -> Result<()> {
        self.write_utf8_truncate(
            path,
            &toml::to_string::<T>(data).with_context(ctx!(
              "Could not serialize toml file {path:?}", ;
              "Ensure that the struct is valid toml",
            ))?,
        )
    }

    fn write_archive<T: Read>(&self, path: &Path, mut data: Archive<T>) -> Result<()> {
        // Verify the path
        if path.exists() {
            bailc!(
                "The path exists.", ;
                "A directory or file exists at {path:?}.", ;
                "Choose a path that is not already taken.",
            );
        }

        let canonical_path = self.truncate_and_canonicalize_folder(path)?;

        // Unpack the archive
        if self.dry_run {
            // Verify the archive is readable
            debug!("Reading the archive");
            for d in data.entries()? {
                let file = d.with_context(ctx!("Error reading an archived example file.", ;
                                        "The example is corrupted.", ))?;

                let archive_path = file.path().with_context(
                    ctx!("Error getting the path of an archived example file.", ;
                                        "The example is corrupted.", ),
                )?;

                let mut copied_path = canonical_path.to_path_buf();
                copied_path.push(&archive_path);
                debug!(
                    "Would have written archived file {:?} to {:?} (dry)",
                    archive_path, copied_path
                );
            }
            Ok(())
        } else {
            data.unpack(&canonical_path).with_context(
                ctx!("Could not unpack an archive to the directory: {:?}", &path;
                            "Ensure that the archive is not corrupt and that \
                            you have permissions to write here.",),
            )?;
            Ok(())
        }
    }

    fn write_utf8_truncate(&self, path: &Path, data: &str) -> Result<()> {
        self.write_bytes_truncate(path, data.as_bytes())
    }

    fn write_bytes_truncate(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        if self.dry_run {
            debug!("Would have written to {path:?} (dry)");
            return Ok(());
        }

        fs::write(self.truncate_and_canonicalize(path)?, bytes).with_context(ctx!(
          "Could not write to the file {path:?}", ;
          "Ensure that you have permissions to write it",
        ))?;

        Ok(())
    }

    fn truncate_and_canonicalize(&self, path: &Path) -> Result<PathBuf> {
        if self.dry_run {
            if let Some(parent) = path.parent() {
                trace!("Would have created {parent:?} (dry)");
            }

            trace!("Would have created {path:?} (dry)");
            return Ok(path.to_path_buf());
        }

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                debug!("Creating directories for {:?}", parent);
            }

            fs::create_dir_all(parent).with_context(ctx!(
              "Could not create parent directories for {parent:?}", ;
              "Ensure that you have sufficient permissions",
            ))?;
        }

        debug!("Creating a file at {:?}", path);
        File::create(path).with_context(ctx!(
           "Could not create {path:?}", ;
           "Ensure that you have sufficient permissions",
        ))?;

        self.canonicalize(path)
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        path.canonicalize().with_context(ctx!(
          "Could not canonicalize {path:?}", ;
          "Ensure that your path is valid",
        ))
    }

    fn truncate_and_canonicalize_folder(&self, path: &Path) -> Result<PathBuf> {
        if self.dry_run {
            debug!("Would have created {path:?} (dry)");
            return Ok(path.to_path_buf());
        }

        debug!("Creating directories for {:?}", path);
        fs::create_dir_all(path).with_context(ctx!(
           "Could not create {path:?}", ;
           "Ensure that you have sufficient permissions",
        ))?;

        path.canonicalize().with_context(ctx!(
          "Could not canonicalize {path:?}", ;
          "Ensure that your path is valid",
        ))
    }

    fn init_git_repository(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            info!("Would have initialized a git repo (dry)");
            return Ok(());
        }

        Repository::init(path)?;
        info!("Successfully created a Git repository");
        Ok(())
    }

    fn set_permissions(&self, path: &Path, perms: u32) -> Result<()> {
        if self.dry_run {
            debug!("Would have made {path:?} executable (dry)");
            return Ok(());
        }

        #[cfg(unix)]
        {
            use std::fs::Permissions;
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, Permissions::from_mode(perms)).with_context(ctx!(
              "Could not make {path:?} executable", ;
             "Ensure that you have sufficient permissions",
            ))
        }
        #[cfg(not(unix))]
        {
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "tests/file_system.rs"]
mod tests;
