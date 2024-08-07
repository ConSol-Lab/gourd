use gourd_lib::config::UserInput;

use crate::config;
use crate::gourd;
use crate::init;
use crate::read_experiment_from_stdout;
use crate::save_gourd_toml;

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
    let conf = config!(&env; "fibonacci"; (
        "input_ten".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            arguments: vec!["10".to_string()],
        },
    ));

    // write the configuration to the tempdir
    let conf_path = save_gourd_toml(&conf, &env.temp_dir);

    let output = gourd!(env; "-c", conf_path.to_str().unwrap(), "run", "local", "--dry", "-s"; "dry run local");

    // check that the output file does not exist
    assert!(read_experiment_from_stdout(&output).is_err());
}
