//! Full workflow integration test.

use std::collections::BTreeMap;

use gourd_lib::config::Label;
use gourd_lib::config::Regex;
use gourd_lib::config::UserInput;

use crate::config;
use crate::gourd;
use crate::init;
use crate::read_experiment_from_stdout;
use crate::save_gourd_toml;

#[test]
fn gourd_run_test() {
    let env = init();

    let conf = config!(env; "slow_fib", "fast_fib", "hello", "fast_fast_fib"; (
        "input_ten".to_string(),
        UserInput {
            file: Some(env.temp_dir.path().join("input_ten")),
            glob: None,fetch: None,git: None,group: None,arguments: vec![],
        }),
        ("input_hello".to_string(),
        UserInput {
            file: Some(env.temp_dir.path().join("input_hello")),
            glob: None,fetch: None,git: None,group: None,arguments: vec![],
        });
        Some(BTreeMap::from([(
            "correct".to_string(),
            Label {
                regex: Regex::from(regex_lite::Regex::new("55").unwrap()),
                priority: 0,
                rerun_by_default: false,
            },
        )]))
    );

    // write the configuration to the tempdir
    let conf_path = save_gourd_toml(&conf, &env.temp_dir);

    let output =
        gourd!(env; "-c", conf_path.to_str().unwrap(), "run", "local", "-s", "-vv"; "run local");

    // check if the output file exists
    let exp = read_experiment_from_stdout(&output).unwrap();
    let output_file = exp.runs.last().unwrap().output_path.clone();
    assert!(output_file.exists());

    // run status
    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s"; "status 1");
    let _o = gourd!(env; "-c", conf_path.to_str().unwrap(), "continue", "-s"; "continue");
    // let _e = read_experiment_from_stdout(&_o).unwrap();
    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s"; "status 2");
    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "rerun", "-r", "0", "-s"; "rerun");

    assert!(!gourd!(env; "cancel").status.success());
}

#[test]
fn gourd_status_test() {
    let env = init();

    let conf1 = config!(env; "slow_fib", "fast_fib", "hello", "fast_fast_fib"; (
        "input_ten".to_string(),
        UserInput {
            file: Some(env.temp_dir.path().join("input_ten")),
            glob: None,fetch: None,git: None,group: None,arguments: vec![],
        }),
        ("input_hello".to_string(),
        UserInput {
            file: Some(env.temp_dir.path().join("input_hello")),
            glob: None,fetch: None,git: None,group: None,arguments: vec![],
        });
        Some(BTreeMap::from([(
            "correct".to_string(),
            Label {
                regex: Regex::from(regex_lite::Regex::new("55").unwrap()),
                priority: 0,
                rerun_by_default: false,
            },
        )]))
    );

    let conf2 = config!(env; "slow_fib"; (
        "input_ten".to_string(),
        UserInput {
            file: Some(env.temp_dir.path().join("input_ten")),
            glob: None,fetch: None,git: None,group: None,arguments: vec![],
        })
    );

    // write the configurations to the tempdir
    let conf1_path = save_gourd_toml(&conf1, &env.temp_dir);

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
    assert_eq!(2, text_out.match_indices("failed").count());
    // 3 programs on input 10 will pass, 1 post on a good output will pass
    assert_eq!(4, text_out.match_indices("success").count());

    // get a new configuration
    let conf2_path = save_gourd_toml(&conf2, &env.temp_dir);

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

    let text_out = std::str::from_utf8(status_2_returned.stdout.as_slice()).unwrap();
    assert_eq!(0, text_out.match_indices("failed").count());
    assert_eq!(1, text_out.match_indices("success").count());

    assert!(!gourd!(env; "cancel").status.success());
}

#[test]
fn gourd_rerun_test() {
    let env = init();

    let conf = config!(env; "slow_fib", "fast_fib", "hello", "fast_fast_fib"; (
        "input_ten".to_string(),
        UserInput {
            file: Some(env.temp_dir.path().join("input_ten")),
            glob: None,fetch: None,git: None,group: None,arguments: vec![],
        }),
        ("input_hello".to_string(),
        UserInput {
            file: Some(env.temp_dir.path().join("input_hello")),
            glob: None,fetch: None,git: None,group: None,arguments: vec![],
        });
        Some(BTreeMap::from([(
            "correct".to_string(),
            Label {
                regex: Regex::from(regex_lite::Regex::new("55").unwrap()),
                priority: 0,
                rerun_by_default: false,
            },
        )]))
    );

    // write the configurations to the tempdir
    let conf_path = save_gourd_toml(&conf, &env.temp_dir);

    let output = gourd!(env; "-c", conf_path.to_str().unwrap(), "run", "local", "-s"; "run local");

    // check if the output file exists
    let exp = read_experiment_from_stdout(&output).unwrap();
    let output_file = exp.runs.last().unwrap().output_path.clone();
    assert!(output_file.exists());

    // run status
    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s"; "status");

    let rerun_output_1 = gourd!(env; "-c", conf_path.to_str().unwrap(), "rerun", "-s"; "rerun");
    let text_err = std::str::from_utf8(rerun_output_1.stderr.as_slice()).unwrap();
    assert!(text_err.contains("2 new runs have been created")); // todo: confirm that "4" is correct

    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "continue", "-s"; "continue");

    let rerun_output_2 = gourd!(env; "-c", conf_path.to_str().unwrap(), "rerun", "-s"; "rerun");
    let text_err = std::str::from_utf8(rerun_output_2.stderr.as_slice()).unwrap();
    assert!(text_err.contains("3 new runs have been created")); // todo: confirm that "4" is correct

    assert!(!gourd!(env; "cancel").status.success());
}
