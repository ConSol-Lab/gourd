#[cfg(unix)]
use std::collections::BTreeMap;
use std::fs;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

use gourd_lib::config::Label;
use gourd_lib::config::UserInput;

use crate::config;
use crate::gourd;
use crate::init;
use crate::save_gourd_toml;

#[test]
fn test_status_afterscript_labels() {
    let env = init();

    let mut label_map = BTreeMap::new();
    label_map.insert(
        String::from("output_was_one"),
        Label {
            regex: gourd_lib::config::Regex::from(".*1.*".parse::<regex_lite::Regex>().unwrap()),
            priority: 2,
            rerun_by_default: false,
        },
    );
    label_map.insert(
        String::from("output_was_not_one"),
        Label {
            regex: gourd_lib::config::Regex::from(".*".parse::<regex_lite::Regex>().unwrap()),
            priority: 1,
            rerun_by_default: true,
        },
    );

    let afterscript_path = env.temp_dir.path().join("afterscript.sh");

    // An afterscript that puts the run output into the afterscript output file.
    // Also attempts to create a directory named using the run output, which should
    // fail for the second fibonacci number that outputs '1', because at that point,
    // an 'output-was-1' directory exists. Thus, the failure handling is tested.

    // Create a new experiment configuration in the tempdir.
    let mut conf = config!(&env; "fibonacci";
        ("input_one".to_string(),
        UserInput {
            input: None,
            arguments: vec!["1".to_string()],
        }),
        ("input_two".to_string(),
        UserInput {
            input: None,
            arguments: vec!["2".to_string()],
        }),
        ("input_five".to_string(),
        UserInput {
            input: None,
            arguments: vec!["5".to_string()],
        })
    );
    conf.labels = Some(label_map);

    conf.programs.0.get_mut("fibonacci").unwrap().afterscript = Some(afterscript_path.clone());

    // write the configuration to the tempdir
    let conf_path = save_gourd_toml(&conf, &env.temp_dir);

    // This afterscript does nothing! And since it doesn't write to a file, it is
    // executed again each time status is queried.
    fs::write(&afterscript_path, String::from("#/bin/bash\n"))
        .expect("Cannot create test afterscript file.");
    fs::set_permissions(&afterscript_path, Permissions::from_mode(0o775))
        .expect("Cannot set afterscript mode to 0755 (executable).");

    let run_out = gourd!(env; "-c", conf_path.to_str().unwrap(), "run", "local", "-s"; "run local");

    let run_stdout_str = String::from_utf8(run_out.stdout).unwrap();
    let run_stderr_str = String::from_utf8(run_out.stderr).unwrap();

    assert!(afterscript_path.exists());

    // Afterscript result fetching fails. The files don't exist!
    assert!(!run_stdout_str.contains("output_was_one"));
    assert!(!run_stdout_str.contains("output_was_not_one"));
    assert!(run_stderr_str.contains("Failed to get status from afterscript 0."));
    assert!(run_stderr_str.contains("Failed to get status from afterscript 1."));
    assert!(run_stderr_str.contains("Failed to get status from afterscript 2."));

    // Now we replace the afterscript file so that it actually writes things.
    // This way, we ensure that the afterscript's final execution happens in
    // gourd status and we can fully test it.
    fs::remove_file(&afterscript_path).unwrap();
    fs::write(
        &afterscript_path,
        String::from("#!/bin/bash\nmkdir ../output-was-`cat $1` && cat $1 > $2"),
    )
    .expect("Cannot create test afterscript file.");
    fs::set_permissions(&afterscript_path, Permissions::from_mode(0o755))
        .expect("Cannot set afterscript mode to 0755 (executable).");

    let status_out = gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s"; "status");

    let status_stdout_str = String::from_utf8(status_out.stdout).unwrap();
    let status_stderr_str = String::from_utf8(status_out.stderr).unwrap();

    assert!(conf.output_path.join("1/fibonacci/0/afterscript").exists());

    assert!(status_stdout_str.contains("output_was_one"));
    assert!(status_stdout_str.contains("output_was_not_one"));
    assert!(status_stderr_str.contains("Failed to get status from afterscript 2."));

    let run_0_status_out =
        gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s", "-i", "0"; "status job 0");
    assert!(String::from_utf8(run_0_status_out.stdout)
        .unwrap()
        .contains("output_was_not_one"));

    let run_1_status_out =
        gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s", "-i", "1"; "status job 1");
    assert!(String::from_utf8(run_1_status_out.stdout)
        .unwrap()
        .contains("output_was_one"));

    let run_2_status_out =
        gourd!(env; "-c", conf_path.to_str().unwrap(), "status", "-s", "-i", "2"; "status job 2");
    assert!(!String::from_utf8(run_2_status_out.stdout)
        .unwrap()
        .contains("output_was"));
    assert!(String::from_utf8(run_2_status_out.stderr)
        .unwrap()
        .contains("Failed to get status from afterscript"));
}
