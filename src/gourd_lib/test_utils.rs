use std::fs::File;
#[cfg(feature = "builtin-examples")]
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use anyhow::bail;
use anyhow::Result;
#[cfg(feature = "builtin-examples")]
use tar::Archive;
use tempdir::TempDir;

use crate::file_system::FileOperations;
use crate::file_system::FileSystemInteractor;

pub const REAL_FS: FileSystemInteractor = FileSystemInteractor { dry_run: false };

// This will come in useful later.
#[allow(dead_code)]
pub const EMPTY_FS: EmptyFilesystem = EmptyFilesystem {};

pub struct EmptyFilesystem;

impl FileOperations for EmptyFilesystem {
    fn read_utf8(&self, _: &Path) -> Result<String> {
        Ok("== Nonsese".to_string())
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        bail!("File not found: {path:?}")
    }

    fn try_read_toml<T: serde::de::DeserializeOwned>(&self, _: &Path) -> Result<T> {
        bail!("File not found")
    }

    fn try_write_toml<T: serde::Serialize>(&self, _: &Path, _: &T) -> Result<()> {
        bail!("File not found")
    }

    fn write_utf8_truncate(&self, _: &Path, _: &str) -> Result<()> {
        bail!("File not found")
    }

    fn write_bytes_truncate(&self, _: &Path, _: &[u8]) -> Result<()> {
        bail!("File not found")
    }

    fn truncate_and_canonicalize(&self, _: &Path) -> Result<PathBuf> {
        bail!("File not found")
    }

    fn truncate_and_canonicalize_folder(&self, _: &Path) -> Result<PathBuf> {
        bail!("File not found")
    }

    fn canonicalize(&self, _: &Path) -> Result<PathBuf> {
        bail!("File not found")
    }

    #[cfg(feature = "builtin-examples")]
    fn write_archive<T: Read>(&self, _path: &Path, _data: Archive<T>) -> Result<()> {
        bail!("File not found")
    }

    fn init_git_repository(&self, _: &Path) -> Result<()> {
        bail!("File not found")
    }

    fn set_permissions(&self, _: &Path, _: u32) -> Result<()> {
        bail!("File not found")
    }

    fn copy(&self, _: &Path, _: &Path) -> Result<PathBuf> {
        bail!("File not found")
    }
}

/// Create a sample config file from a string, used in testing.
///
/// If you need to test the Config struct you can use this function to create a
/// sample config file, get the returned path to it and then parse it.
/// ```ignore
/// # use gourd_lib::test_utils::create_sample_toml;
/// let (file_pb, dir) = create_sample_toml("
/// [section]
/// key = \"value\"
/// ");
/// // use the test file ...
/// # assert!(file_pb.exists());
/// # assert!(&dir.path().exists());
/// // ... and then clean up.
/// # let p = dir.path().to_path_buf();
/// dir.close().unwrap();
/// # assert!(!p.exists());
/// # assert!(!file_pb.exists());
/// ```
pub fn create_sample_toml(config_contents: &str) -> (PathBuf, TempDir) {
    let dir = TempDir::new("config_folder").expect("A temp folder could not be created.");
    let file_pb = dir.path().join("file.toml");
    let mut file = File::create(file_pb.as_path()).expect("A file could not be created.");
    file.write_all(config_contents.as_bytes())
        .expect("The test file could not be written.");
    (file_pb, dir)
}
