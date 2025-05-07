use crate::config;
use crate::gourd;
use crate::init;

#[test]
fn test_analyse_csv() {
    let env = init();

    // Create a new experiment configuration in the tempdir.
    let (_, conf_path) = config(&env, "./src/integration/configurations/single_run.toml").unwrap();

    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(),
        "run", "local", "-s"; "run local");

    let output = gourd!(env; "-c", conf_path.to_str().unwrap(),
        "analyse", "table", "-s", "--format=program,exit-code,afterscript"; "analyse csv");

    let table = std::str::from_utf8(&output.stdout).unwrap();
    assert!(table.contains("0"));
    assert!(table.contains("fibonacci"));
    assert!(table.contains('0'));
    assert!(table.contains("N/A"));
}

#[test]
fn test_analyse_csv_file() {
    let env = init();

    // Create a new experiment configuration in the tempdir.
    let (conf, conf_path) =
        config(&env, "./src/integration/configurations/single_run.toml").unwrap();

    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(),
        "run", "local", "-s"; "run local");

    let out_path = conf.experiments_folder.join("analysis_1.csv");
    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(),
        "analyse", "table", "-s", "-o", out_path.to_str().unwrap(); "analyse csv");

    assert!(out_path.exists());
}
