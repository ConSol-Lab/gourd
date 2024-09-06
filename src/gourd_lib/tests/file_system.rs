#![cfg(feature = "builtin-examples")]

use std::fs;
use std::fs::File;

use tar::Archive;

use crate::config::Config;
use crate::experiment::Environment;
use crate::file_system::FileOperations;
use crate::file_system::FileSystemInteractor;

#[test]
fn try_read_toml_test() {
    let tempdir = tempdir::TempDir::new("fs_test").unwrap();
    let filepath = tempdir.path().join("x.toml");
    let fsi = FileSystemInteractor { dry_run: false };

    fs::write(&filepath, "invalid toml goes here").unwrap();
    assert!(fsi.try_read_toml::<Config>(&filepath).is_err());
}

#[test]
fn try_write_toml_test() {
    let tempdir = tempdir::TempDir::new("fs_test").unwrap();
    let filepath = tempdir.path().join("x.toml");
    let fsi = FileSystemInteractor { dry_run: false };

    // enums cannot be serialised
    assert!(fsi
        .try_write_toml::<Environment>(&filepath, &Environment::Local)
        .is_err());
}

#[test]
fn write_archive_test() {
    let tempdir = tempdir::TempDir::new("fs_test").unwrap();
    let filepath = tempdir.path().join("archive.tar");
    fs::write(&filepath, "invalid archive goes here").unwrap();

    let archive = Archive::new(File::open(&filepath).unwrap());
    let fsi = FileSystemInteractor { dry_run: false };

    assert!(fsi
        .write_archive(tempdir.path(), archive)
        .is_err_and(|e| e.to_string().contains("directory or file exists")));

    let archive = Archive::new(File::open(&filepath).unwrap());

    assert!(fsi
        .write_archive(&tempdir.path().join("some_dir"), archive)
        .is_err_and(|e| e
            .to_string()
            .contains("Ensure that the archive is not corrupt")));

    let archive = Archive::new(File::open(&filepath).unwrap());
    let fsi = FileSystemInteractor { dry_run: true };

    assert!(fsi
        .write_archive(&tempdir.path().join("some_dir"), archive)
        .is_err());
}

#[test]
fn set_permissions_test() {
    let tempdir = tempdir::TempDir::new("fs_test").unwrap();
    let filepath = tempdir.path().join("x.toml");
    fs::write(&filepath, "").unwrap();

    let fsi = FileSystemInteractor { dry_run: true };
    fsi.set_permissions(&filepath, 0o755).unwrap();

    let fsi = FileSystemInteractor { dry_run: false };
    fsi.set_permissions(&filepath, 0o755).unwrap();
}
