#[cfg(unix)]
use crate::config;
use crate::gourd;
use crate::init;

#[test]
fn test_status_afterscript_labels() {
    let env = init();

    // Create a new experiment configuration in the tempdir.
    let (_conf, conf_path) = config(
        &env,
        "./src/integration/configurations/wrong_afterscript.toml",
    )
    .unwrap();

    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "run", "local", "-s"; "run local");
    let run_out = gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s"; "status");

    let run_stdout_str = String::from_utf8(run_out.stdout).unwrap();

    // since the afterscript does not output to a file, no labels should be present.
    assert!(
        !run_stdout_str.contains("output_was_one"),
        "run_stdout: {run_stdout_str}"
    );
    assert!(
        !run_stdout_str.contains("output_was_not_one"),
        "run_stdout: {run_stdout_str}"
    );
}

#[test]
fn afterscript_test_2() {
    let env = init();

    // Create a new experiment configuration in the tempdir.
    let (_conf, conf_path) = config(&env, "./src/integration/configurations/numeric.toml").unwrap();

    let _ = gourd!(env; "-c", conf_path.to_str().unwrap(), "run", "local", "-s"; "run local");
    let status_out = gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s"; "status");

    let status_stdout_str = String::from_utf8(status_out.stdout).unwrap();

    assert!(
        status_stdout_str.contains("output_was_one"),
        "status_stdout: {status_stdout_str}"
    );
    assert!(
        status_stdout_str.contains("output_was_not_one"),
        "status_stdout: {status_stdout_str}"
    );
}
