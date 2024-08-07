use std::path::PathBuf;

use tempdir::TempDir;

use super::*;
use crate::test_utils::REAL_FS;

#[test]
fn latest_id_invalid_folder() {
    let error = Experiment::latest_id_from_folder(Path::new(
        "INVALID PATH
:)",
    ));
    assert!(error.is_err());
    assert_eq!(
        format!("{}", error.unwrap_err().root_cause()),
        "No such file or directory (os error 2)"
    );
}

#[test]
fn latest_id_correct() {
    let tempdir = TempDir::new("tests").unwrap();
    let mut config: Config = Config::from_file(
        Path::new("src/gourd/experiments/tests/test_resources/config_id_testing.toml"),
        &REAL_FS,
    )
    .unwrap();
    config.output_path = PathBuf::from(tempdir.path());
    config.metrics_path = PathBuf::from(tempdir.path());
    config.experiments_folder = PathBuf::from(tempdir.path());

    // test other files in dir that should be ignored
    fs::create_dir(tempdir.path().join("39.lock")).unwrap();
    fs::create_dir(tempdir.path().join("393")).unwrap();
    fs::create_dir(tempdir.path().join("directory")).unwrap();
    fs::write(tempdir.path().join("19"), []).unwrap();
    fs::write(tempdir.path().join("8.lock.bkp"), []).unwrap();

    // latest_id_from_folder only works if the experiments are actually saved
    for _ in 1..=8 {
        Experiment::from_config(&config, Local::now(), Environment::Local, &REAL_FS)
            .unwrap()
            .save(&REAL_FS)
            .unwrap();
    }

    let id = Experiment::latest_id_from_folder(tempdir.path()).unwrap();
    assert_eq!(id, Some(8));
}
