use std::io::Read;
use std::io::Write;
use std::process::Stdio;
use std::string::String;

use gourd_lib::config::UserInput;

use crate::config;
use crate::gourd;
use crate::init;
use crate::read_experiment_from_stdout;
use crate::save_gourd_toml;

#[test]
fn test_dry_one_run() {
    let env = init();
    let conf = config!(&env; "fibonacci"; (
        "input_ten".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            arguments: vec!["10".to_string()],
        },
    ));
    let conf_path = save_gourd_toml(&conf, &env.temp_dir);

    let output = gourd!(&env; "-c", conf_path.to_str().unwrap(), "run", "local", "-s"; "run local");
    let mut exp = read_experiment_from_stdout(&output).unwrap();
    assert_eq!(exp.runs.len(), 1);

    let _ =
        gourd!(&env; "-c", conf_path.to_str().unwrap(), "rerun", "-s", "-r", "0"; "rerun local");

    let _ = gourd!(&env; "-c", conf_path.to_str().unwrap(), "continue", "-s"; "continue");
    exp = read_experiment_from_stdout(&output).unwrap();
    assert_eq!(exp.runs.len(), 2);
}

#[test]
fn test_two_one_run() {
    let env = init();
    let conf = config!(&env; "fibonacci"; (
        "input_ten".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            arguments: vec!["10".to_string()],
        },
    ));
    let conf_path = save_gourd_toml(&conf, &env.temp_dir);

    let output = gourd!(&env; "-c", conf_path.to_str().unwrap(), "run", "local", "-s"; "run local");
    let mut exp = read_experiment_from_stdout(&output).unwrap();
    assert_eq!(exp.runs.len(), 1);

    let _ =
        gourd!(&env; "-c", conf_path.to_str().unwrap(), "rerun", "-s", "-r", "0"; "rerun local");

    let _ = gourd!(&env; "-c", conf_path.to_str().unwrap(), "continue", "-s"; "continue");
    exp = read_experiment_from_stdout(&output).unwrap();
    assert_eq!(exp.runs.len(), 2);
}

#[test]
fn test_setting_resource_limits() {
    let env = init();
    let conf = config!(&env; "fibonacci", "fast_fib", "fast_fast_fib";
        ("input_one".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            arguments: vec!["1".to_string()],
        }),
        ("input_two".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,arguments: vec!["2".to_string()],
        }),
        ("input_five".to_string(),
        UserInput {
            file: None,
            glob: None,
            fetch: None,
            arguments: vec!["5".to_string()],
        })
    );

    let conf_path = save_gourd_toml(&conf, &env.temp_dir);

    let experiment_path = conf.experiments_folder.join("1.lock");
    assert!(!experiment_path.exists());

    let _ = gourd!(&env; "-c", conf_path.to_str().unwrap(), "run", "local"; "run local");

    // Invalid arguments cause 3 runs to fail, we are rerunning them.

    let gourd_command = env.gourd_path.to_str().unwrap().to_owned()
        + " -c "
        + conf_path.to_str().unwrap()
        + " rerun";
    // This is needed to simulate a TTY.
    // The inquire library doesn't work when it does not detect a terminal.
    let mut gourd = fake_tty::command(&gourd_command, None)
        .expect("Could not create a fake TTY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not spawn gourd");

    {
        let stdin = gourd.stdin.as_mut().unwrap();

        // > Rerun only failed (3 runs)
        // Rerun all finished (6 runs)

        // Select 'only failed'
        stdin.write_all(b"\n").unwrap();
    }
    // block drops stdin/out

    let mut s = String::new();

    gourd.stdout.unwrap().read_to_string(&mut s).unwrap();

    assert!(s.contains("failed"));
    assert!(s.contains("all finished"));
    assert!(s.contains(" new runs have been created"));

    // Now the runs are already scheduled. Let's try rerun again.

    let gourd_command = env.gourd_path.to_str().unwrap().to_owned()
        + " -c "
        + conf_path.to_str().unwrap()
        + " rerun";
    // This is needed to simulate a TTY.
    // The inquire library doesn't work when it does not detect a terminal.
    let mut gourd = fake_tty::command(&gourd_command, None)
        .expect("Could not create a fake TTY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not spawn gourd");

    {
        let stdin = gourd.stdin.as_mut().unwrap();

        // > Rerun only failed (0 runs)
        // Rerun all finished (3 runs)

        // Select 'only failed'
        let _ = stdin.write_all(b"\n");
    }

    let mut s = String::new();

    gourd.stdout.unwrap().read_to_string(&mut s).unwrap();

    assert!(s.contains("failed (0 runs)"));
    assert!(s.contains("all finished (3 runs)"));
    assert!(s.contains("No new runs to schedule"));

    // Now try to rerun an already rerun run

    let gourd_command = env.gourd_path.to_str().unwrap().to_owned()
        + " -c "
        + conf_path.to_str().unwrap()
        + " rerun -r 1";
    // This is needed to simulate a TTY.
    // The inquire library doesn't work when it does not detect a terminal.
    let gourd = fake_tty::command(&gourd_command, None)
        .expect("Could not create a fake TTY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not spawn gourd");

    let mut s = String::new();

    gourd.stdout.unwrap().read_to_string(&mut s).unwrap();

    assert!(s.contains("already rerun"));
}
