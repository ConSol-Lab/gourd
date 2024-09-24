use crate::config;
use crate::gourd;
use crate::init;
use crate::read_experiment_from_stdout;

#[test]
fn test_no_config() {
    let env = init();

    let output = gourd!(env; "run", "local", "--dry");

    // there's no gourd.toml, so this should fail
    assert!(!output.status.success());
}

#[test]
fn test_dry_one_run() {
    let env = init();

    // Create a new experiment configuration in the tempdir.
    let (_conf, conf_path) =
        config(&env, "./src/integration/configurations/single_run.toml").unwrap();

    let output = gourd!(env; "-c", conf_path.to_str().unwrap(), "run", "local", "--dry", "-s"; "dry run local");

    // check that the output file does not exist
    assert!(read_experiment_from_stdout(&output).is_err());
}
