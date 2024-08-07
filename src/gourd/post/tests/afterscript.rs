use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use gourd_lib::config::Config;
use gourd_lib::experiment::Environment;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileSystemInteractor;
use tempdir::TempDir;

use crate::experiments::ExperimentExt;
use crate::post::afterscript::run_afterscript_for_run;
use crate::post::labels::assign_label;

const PRE_PROGRAMMED_SH_SCRIPT: &str = r#"
#!/bin/sh
tr '[a-z]' '[A-Z]' <$1 >$2
"#;

const PRE_PROGRAMMED_RESULTS: &str = r#"[package]
name = "gourd"
edition = "2021"

[dependencies]
"#;

#[test]
fn test_run_afterscript_for_run_good_weather() {
    let tmp_dir = TempDir::new("testing").unwrap();

    let results_path = tmp_dir.path().join("results.toml");
    let results_file = fs::File::create(&results_path).unwrap();
    fs::write(&results_path, PRE_PROGRAMMED_RESULTS).unwrap();

    let afterscript_path = tmp_dir.path().join("afterscript.sh");
    let afterscript_file = fs::File::create(&afterscript_path).unwrap();
    fs::write(&afterscript_path, PRE_PROGRAMMED_SH_SCRIPT).unwrap();

    let output_path = tmp_dir.path().join("afterscript_output.toml");

    assert!(run_afterscript_for_run(
        &afterscript_path,
        &results_path,
        &output_path,
        tmp_dir.path()
    )
    .is_ok());

    let mut contents = String::new();
    assert!(fs::File::open(output_path)
        .unwrap()
        .read_to_string(&mut contents)
        .is_ok());
    assert_eq!(contents, PRE_PROGRAMMED_RESULTS.to_ascii_uppercase());

    drop(results_file);
    drop(afterscript_file);

    assert!(tmp_dir.close().is_ok());
}

#[test]
fn test_run_afterscript_for_run_bad_weather() {
    let tmp_dir = TempDir::new("testing").unwrap();

    let results_path = tmp_dir.path().join("results.toml");
    let results_file = File::create(&results_path).unwrap();
    fs::write(&results_path, PRE_PROGRAMMED_RESULTS).unwrap();

    let output_path = tmp_dir.path().join("afterscript_output.toml");

    assert!(run_afterscript_for_run(
        &PathBuf::from(""),
        &results_path,
        &output_path,
        tmp_dir.path()
    )
    .is_err());

    drop(results_file);

    assert!(tmp_dir.close().is_ok());
}

#[test]
fn test_add_label_to_run() {
    let fs = FileSystemInteractor { dry_run: true };
    let dir = TempDir::new("config_folder").expect("A temp folder could not be created.");
    let file_pb = dir.path().join("file.toml");
    let config_contents = r#"
             output_path = "./goose"
             metrics_path = "./🪿/"
             experiments_folder = "/tmp/gourd/experiments/"
             [program.a]
             binary = "/bin/sleep"
             arguments = []
             afterscript = "/bin/echo"
             [input.b]
             arguments = ["1"]
             [input.c]
             arguments = ["2"]
             [label.found_hello]
             priority = 0
             regex = "hello"
             [label.found_world]
             priority = 1
             regex = "world"
         "#;
    let mut file = File::create(file_pb.as_path()).expect("A file could not be created.");
    file.write_all(config_contents.as_bytes())
        .expect("The test file could not be written.");
    let mut after_file =
        File::create(dir.path().join("after.txt")).expect("A file could not be created.");
    after_file
        .write_all("hello".as_bytes())
        .expect("The test file could not be written.");

    let conf = Config::from_file(file_pb.as_path(), &fs).unwrap();
    let exp =
        Experiment::from_config(&conf, chrono::Local::now(), Environment::Local, &fs).unwrap();
    assert!(conf.labels.is_some());
    assert_eq!(
        assign_label(&exp, &dir.path().join("after.txt"), &fs).expect("tested fn failed"),
        Some("found_hello".to_string())
    );

    after_file
        .write_all("hello world".as_bytes())
        .expect("The test file could not be written.");

    assert_eq!(
        assign_label(&exp, &dir.path().join("after.txt"), &fs).expect("tested fn failed"),
        Some("found_world".to_string())
    );
}
