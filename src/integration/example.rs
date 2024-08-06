use gourd_lib::config::UserInput;

use crate::config;
use crate::gourd;
use crate::init;
use crate::read_experiment_from_stdout;
use crate::save_gourd_toml;

#[test]
fn test_one_run() {
    let env = init();

    // Create a new experiment configuration in the tempdir.
    let conf = config!(env; "fibonacci"; (
        "input_ten".to_string(),
        UserInput {
            file: None,
            arguments: vec!["10".to_string()],
        },
    ));

    // write the configuration to the tempdir
    let conf_path = save_gourd_toml(&conf, &env.temp_dir);

    let output = gourd!(env; "-c", conf_path.to_str().unwrap(), "run", "local", "-s"; "run local");

    // check if the output file exists
    let exp = read_experiment_from_stdout(&output).unwrap();
    let output_file = exp.runs.last().unwrap().output_path.clone();
    assert!(output_file.exists());

    // the program in this case is fibonacci, so the output should be 55
    let output = std::fs::read_to_string(output_file).unwrap();
    assert_eq!(output.trim(), "55");
}
