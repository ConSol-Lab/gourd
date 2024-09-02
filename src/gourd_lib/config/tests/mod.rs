extern crate tempdir;

use std::collections::BTreeMap;
use std::fs::File;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs::Permissions;
use std::io::Write;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use tempdir::TempDir;

use crate::config::Config;
use crate::config::UserInput;
use crate::config::UserProgram;
use crate::constants::WRAPPER_DEFAULT;
use crate::test_utils::create_sample_toml;
use crate::test_utils::REAL_FS;

/// This test will fail if the semantics of the config struct are changed.
/// If this is the case, update the documentation and make sure that the
/// rest of the application reflects these changes.
#[test]
fn breaking_changes_config_struct() {
    #[allow(clippy::unnecessary_operation)]
    Config {
        output_path: PathBuf::from(""),
        metrics_path: PathBuf::from(""),
        experiments_folder: PathBuf::from(""),
        wrapper: "".to_string(),
        inputs: BTreeMap::default(),
        parameters: None,
        programs: BTreeMap::default(),
        input_schema: None,
        slurm: None,
        resource_limits: None,
        labels: Some(BTreeMap::new()),
        warn_on_label_overlap: false,
    };
}

/// This test will fail if the semantics of the config file are changed.
/// See above. Is this a valid reason for the user to update their old files?
/// If you add something to the struct, add it here too.
#[test]
fn breaking_changes_config_file_all_values() {
    let dir = TempDir::new("config_folder").expect("A temp folder could not be created.");
    let file_pathbuf = dir.path().join("file.toml");

    let config_contents = r#"
        output_path = "./ginger_root"
        metrics_path = "./vulfpeck/"
        experiments_folder = "./parcels/"
        wrapper = "gourd_wrapper"

        [program]

        [input]
    "#;
    let mut file = File::create(file_pathbuf.as_path()).expect("A file could not be created.");
    file.write_all(config_contents.as_bytes())
        .expect("The test file could not be written.");

    assert_eq!(
        Config {
            output_path: PathBuf::from("./ginger_root/"),
            metrics_path: PathBuf::from("./vulfpeck"),
            experiments_folder: PathBuf::from("./parcels/"),
            wrapper: "gourd_wrapper".to_string(),
            inputs: BTreeMap::default(),
            parameters: None,
            programs: BTreeMap::default(),
            input_schema: None,
            slurm: None,
            resource_limits: None,
            labels: None,
            warn_on_label_overlap: false,
        },
        Config::from_file(file_pathbuf.as_path(), &REAL_FS).expect("Unexpected config read error.")
    );
    dir.close().unwrap();
}

/// This test will fail if the semantics of all REQUIRED values in the config
/// file are changed. See above. If you add something to the struct, add it here
/// too.
#[test]
fn breaking_changes_config_file_required_values() {
    let (file_pb, dir) = create_sample_toml(
        r#"
        output_path = "./ginger_root"
        metrics_path = "./vulfpeck/"
        experiments_folder = ""

        [input]

        [program]
    "#,
    );

    assert_eq!(
        Config {
            output_path: PathBuf::from("./ginger_root/"),
            metrics_path: PathBuf::from("./vulfpeck"),
            experiments_folder: PathBuf::from(""),
            wrapper: "gourd_wrapper".to_string(),
            inputs: BTreeMap::default(),
            parameters: None,
            programs: BTreeMap::default(),
            input_schema: None,
            slurm: None,
            resource_limits: None,
            labels: None,
            warn_on_label_overlap: false,
        },
        Config::from_file(file_pb.as_path(), &REAL_FS).expect("Unexpected config read error.")
    );
    dir.close().unwrap();
}

#[test]
fn config_nonexistent_file() {
    let dir = TempDir::new("config_folder").unwrap();
    let file_pathbuf = dir.path().join("file.toml");

    if Config::from_file(file_pathbuf.as_path(), &REAL_FS).is_ok() {
        panic!("Error expected.")
    }

    dir.close().unwrap();
}

// Tests on a file without read permissions.
// The test does not run on Windows because we access Unix-style permissions
// here.
#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn config_unreadable_file() {
    let dir = TempDir::new("config_folder").expect("A temp folder could not be created.");
    let file_pathbuf = dir.path().join("file.toml");
    let file = File::create(file_pathbuf.as_path()).expect("A file could not be created.");
    file.set_permissions(Permissions::from_mode(0o000))
        .expect("Could not set permissions of 'unreadable' test file to 000.");

    if Config::from_file(file_pathbuf.as_path(), &REAL_FS).is_ok() {
        panic!("Error expected.")
    }
    dir.close().unwrap();
}

#[test]
fn config_unparseable_file() {
    let dir = TempDir::new("config_folder").expect("A temp folder could not be created.");
    let file_pathbuf = dir.path().join("file.toml");

    File::create(file_pathbuf.as_path()).expect("A file folder could not be created.");

    if Config::from_file(file_pathbuf.as_path(), &REAL_FS).is_ok() {
        panic!("Error expected.")
    }
    dir.close().unwrap();
}

#[test]
fn config_ok_file() {
    let (file_pb, dir) = create_sample_toml(
        &toml::to_string(&Config::default())
            .expect("The default Config could not be serialized for testing."),
    );

    assert_eq!(
        Config::default(),
        Config::from_file(file_pb.as_path(), &REAL_FS).expect("Unexpected config read error.")
    );
    dir.close().unwrap();
}

#[test]
fn disallow_glob_names() {
    let (file_pb, dir) = create_sample_toml(
        r#"
            output_path = "./ginger_root"
            metrics_path = "./vulfpeck/"
            experiments_folder = ""

            [input.test_glob_]
            arguments = []

            [program]
        "#,
    );
    assert!(format!("{:?}", Config::from_file(file_pb.as_path(), &REAL_FS)).contains("_glob_"));
    dir.close().unwrap();
}

#[test]
fn test_regex_that_do_match() {
    let (file_pb, _dir) = create_sample_toml(
        r#"
            output_path = "./ginger_root"
            metrics_path = "./vulfpeck/"
            experiments_folder = ""
            [label.stefan]
            regex = "[a-zA-Z0-9]+ loves stefan"
            priority = 42
            [program]
            [input]
        "#,
    );
    let config =
        Config::from_file(file_pb.as_path(), &REAL_FS).expect("Unexpected config read error.");
    assert!(
        regex_lite::Regex::new(config.labels.unwrap().get("stefan").unwrap().regex.as_str())
            .unwrap()
            .is_match("18C loves stefan")
    );
}

#[test]
fn test_invalid_regex_gives_error() {
    let (file_pathbuf, _dir) = create_sample_toml(
        r#"
            output_path = "./ginger_root"
            metrics_path = "./vulfpeck/"
            experiments_folder = ""
            [label.stefan]
            regex = "{{{{{{{{{{{{{{{{ i didnt pass acc"
            priority = 42
            [program]
            [input]
        "#,
    );
    assert!(Config::from_file(file_pathbuf.as_path(), &REAL_FS).is_err());
}

#[test]
fn parse_valid_escape_hatch_file() {
    let dir = TempDir::new("config_folder").expect("A temp folder could not be created.");
    let f1 = dir.path().join("file1.toml");
    let f2 = dir.path().join("file2.toml");

    let mut file1 = File::create(f1.as_path()).expect("A file could not be created.");
    let mut file2 = File::create(f2.as_path()).expect("A file could not be created.");

    file1
        .write_all(
            format!(
                "
              output_path = \"{}/42\"
              metrics_path = \"{}/43\"
              experiments_folder = \"{}/44\"
              input_schema = \"{}\"
              [program.x]
              binary = \"/bin/sleep\"
              [input]
            ",
                dir.path().to_str().unwrap(),
                dir.path().to_str().unwrap(),
                dir.path().to_str().unwrap(),
                f2.as_path().to_str().unwrap()
            )
            .as_bytes(),
        )
        .expect("The test file could not be written.");

    file2
        .write_all(
            "
    [[input]]
    arguments = [\"hello\"]
    [[input]]
    arguments = [\"hi\"]
    "
            .as_bytes(),
        )
        .unwrap();

    let c1 = Config::from_file(f1.as_path(), &REAL_FS).expect("Unexpected config read error.");
    let c2 = Config {
        output_path: dir.path().join("42"),
        metrics_path: dir.path().join("43"),
        experiments_folder: dir.path().join("44"),
        parameters: None,
        programs: vec![(
            "x".to_string(),
            UserProgram {
                binary: Some(PathBuf::from("/bin/sleep".to_string())),
                fetch: None,
                git: None,
                arguments: vec![],
                afterscript: None,
                resource_limits: None,
                next: vec![],
            },
        )]
        .into_iter()
        .collect::<BTreeMap<String, UserProgram>>(),
        inputs: vec![
            (
                format!(
                    "0{}{}",
                    crate::constants::INTERNAL_PREFIX,
                    crate::constants::INTERNAL_SCHEMA_INPUTS
                ),
                UserInput {
                    file: None,
                    glob: None,
                    fetch: None,
                    group: None,
                    arguments: vec!["hello".to_string()],
                },
            ),
            (
                format!(
                    "1{}{}",
                    crate::constants::INTERNAL_PREFIX,
                    crate::constants::INTERNAL_SCHEMA_INPUTS
                ),
                UserInput {
                    file: None,
                    glob: None,
                    fetch: None,
                    group: None,
                    arguments: vec!["hi".to_string()],
                },
            ),
        ]
        .into_iter()
        .collect::<BTreeMap<String, UserInput>>(),
        input_schema: None,
        slurm: None,
        resource_limits: None,
        wrapper: WRAPPER_DEFAULT(),
        labels: None,
        warn_on_label_overlap: false,
    };
    assert_eq!(c1, c2);
}
