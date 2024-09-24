//! Full workflow integration test.

use crate::config;
use crate::gourd;
use crate::init;
use crate::read_experiment_from_stdout;

#[test]
fn gourd_run_test() {
    let env = init();

    let (_conf, conf_path) =
        config(&env, "./src/integration/configurations/using_labels.toml").unwrap();

    let output =
        gourd!(env; "-c", conf_path.to_str().unwrap(), "run", "local", "-s", "-vv"; "run local");

    // check if the output file exists
    let exp = read_experiment_from_stdout(&output).unwrap();
    let output_file = exp.runs.last().unwrap().output_path.clone();
    assert!(output_file.exists());

    // run status
    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s"; "status 1");
    let _o = gourd!(env; "-c", conf_path.to_str().unwrap(), "continue", "-s"; "continue");

    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s"; "status 2");
    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "rerun", "-r", "0", "-s"; "rerun");

    assert!(!gourd!(env; "cancel").status.success());
}

#[test]
fn gourd_status_test() {
    let env = init();

    let (_conf1, conf1_path) =
        config(&env, "./src/integration/configurations/using_labels.toml").unwrap();

    let (_conf2, conf2_path) =
        config(&env, "./src/integration/configurations/slow_ten.toml").unwrap();

    let output = gourd!(env; "-c", conf1_path.to_str().unwrap(), "run", "local", "-s"; "run local");

    // check if the output file exists
    let exp = read_experiment_from_stdout(&output).unwrap();
    let output_file = exp.runs.last().unwrap().output_path.clone();
    assert!(output_file.exists());

    // run status
    let status_1_returned =
        gourd!(env; "-c", conf1_path.to_str().unwrap(), "status", "-s"; "status 1");

    let text_err = std::str::from_utf8(status_1_returned.stderr.as_slice()).unwrap();
    assert_eq!(
        text_err,
        "info: Displaying the status of jobs for experiment 1\n"
    );

    let text_out = std::str::from_utf8(status_1_returned.stdout.as_slice()).unwrap();
    // 3 programs on input "hello" will fail, 1 post on a failed will fail
    assert_eq!(1, text_out.match_indices("failed").count());
    // 3 programs on input 10 will pass, 1 post on a good output will pass
    assert_eq!(4, text_out.match_indices("success").count()); // TODO: fix

    let output = gourd!(env; "-c", conf2_path.to_str().unwrap(), "run", "local", "-s"; "run local");

    // check if the output file exists for experiment 2
    let exp = read_experiment_from_stdout(&output).unwrap();
    let output_file = exp.runs.last().unwrap().output_path.clone();
    assert!(output_file.exists());

    // run status for the new experiment
    let status_2_returned =
        gourd!(env; "-c", conf2_path.to_str().unwrap(), "status", "-s"; "status 2");

    let text_err = std::str::from_utf8(status_2_returned.stderr.as_slice()).unwrap();
    assert_eq!(
        text_err,
        "info: Displaying the status of jobs for experiment 2\n"
    );

    let _text_out = std::str::from_utf8(status_2_returned.stdout.as_slice()).unwrap();
    assert_eq!(0, text_out.match_indices("failed").count()); // TODO: fix
    assert_eq!(1, text_out.match_indices("success").count());

    assert!(!gourd!(env; "cancel").status.success());
}

#[test]
fn gourd_rerun_test() {
    let env = init();

    let (_conf, conf_path) =
        config(&env, "./src/integration/configurations/using_labels.toml").unwrap();

    let output = gourd!(env; "-c", conf_path.to_str().unwrap(), "run", "local", "-s"; "run local");

    // check if the output file exists
    let exp = read_experiment_from_stdout(&output).unwrap();
    let output_file = exp.runs.last().unwrap().output_path.clone();
    assert!(output_file.exists());

    // run status
    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s"; "status");

    let rerun_output_1 = gourd!(env; "-c", conf_path.to_str().unwrap(), "rerun", "-s"; "rerun");
    let text_err = std::str::from_utf8(rerun_output_1.stderr.as_slice()).unwrap();
    assert!(text_err.contains("2 new runs have been created")); // TODO: fix

    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "continue", "-s"; "continue");

    let rerun_output_2 = gourd!(env; "-c", conf_path.to_str().unwrap(), "rerun", "-s"; "rerun");
    let text_err = std::str::from_utf8(rerun_output_2.stderr.as_slice()).unwrap();
    assert!(text_err.contains("3 new runs have been created")); // TODO: confirm
                                                                // that "4" is correct

    assert!(!gourd!(env; "cancel").status.success());
}
