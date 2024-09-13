use std::collections::BTreeMap;
use std::env;
use std::fs;

use gourd_lib::config::UserInput;
use gourd_lib::config::UserProgram;
use gourd_lib::file_system::FileSystemInteractor;

use super::*;
use crate::status::DynamicStatus;
use crate::test_utils::create_sample_experiment;
use crate::test_utils::get_compiled_example;
use crate::test_utils::REAL_FS;
use crate::wrapper::wrap;

/// This test will generate an ARM binary and check if [crate::wrapper::wrap]
/// rightfully rejects it.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn non_matching_arch() {
    use crate::test_utils::create_sample_experiment;
    use crate::test_utils::REAL_FS;

    const NO_STD_INFINITE_LOOP_RS: &str = include_str!("test_resources/panic_returner.rs");

    Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg("thumbv7em-none-eabihf")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    let (out, tmp) = get_compiled_example(
        NO_STD_INFINITE_LOOP_RS,
        Some(vec!["--target=thumbv7em-none-eabihf"]),
    );

    let input = tmp.join("test1");

    fs::write(&input, "4").unwrap();

    let mut first = BTreeMap::new();

    first.insert(
        "any".to_string(),
        UserProgram {
            binary: Some(out),
            fetch: None,
            git: None,
            env: None,
            arguments: vec![],
            afterscript: None,
            resource_limits: None,
            next: vec![],
        },
    );

    let mut second = BTreeMap::new();

    second.insert(
        "test1".to_string(),
        UserInput {
            file: Some(input.clone()),
            glob: None,
            arguments: vec![],
            fetch: None,
            group: None,
            git: None,
        },
    );

    let (mut experiment, _) = create_sample_experiment(first, second);
    let status = experiment
        .status(&FileSystemInteractor { dry_run: true })
        .unwrap();

    match wrap(&mut experiment, &status, "x86_64", &REAL_FS) {
        Err(err) => {
            assert!(format!("{}", err.root_cause()).contains("not match the expected architecture"))
        }

        e => {
            panic!(
                "Did not return the correct architecture mismatch, was: {:?}",
                e
            );
        }
    }
}

/// This test will generate a binary and of the target architecture and check if
/// [crate::wrapper::wrap] accepts it and generates correct commands.
#[test]
fn matching_arch() {
    const NUM_RETURNER_RS: &str = include_str!("test_resources/num_returner.rs");

    let (out, tmp) = get_compiled_example(NUM_RETURNER_RS, None);
    let input = tmp.join("test1");

    fs::write(&input, "4").unwrap();

    let mut first = BTreeMap::new();

    first.insert(
        "any".to_string(),
        UserProgram {
            binary: Some(out.clone()),
            fetch: None,
            git: None,
            env: None,
            arguments: vec![],
            afterscript: None,
            resource_limits: None,
            next: vec![],
        },
    );

    let mut second = BTreeMap::new();

    second.insert(
        "test1".to_string(),
        UserInput {
            file: Some(input.clone()),
            glob: None,
            fetch: None,
            group: None,
            arguments: vec![],
            git: None,
        },
    );

    let (mut experiment, conf) = create_sample_experiment(first, second);

    let status = experiment
        .status(&FileSystemInteractor { dry_run: true })
        .unwrap();

    let cmds = wrap(&mut experiment, &status, env::consts::ARCH, &REAL_FS).unwrap();

    assert_eq!(1, cmds.len());
    assert_eq!(
        format!("{:?}", cmds[0]),
        format!(
            "{:?}",
            Command::new(conf.wrapper)
                .arg(experiment.file())
                .arg("0")
                .arg("0")
        )
    );
}
