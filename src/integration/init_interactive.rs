#![cfg(unix)]

use std::fs;
use std::path::PathBuf;

use crate::gourd;
use crate::init;

#[test]
fn test_init_script_dry() {
    let env = init();

    let path = env.temp_dir.path().join("folder");
    assert!(!path.exists());

    let path_str = path.to_str().unwrap();
    let output = gourd!(env; "init", "--dry", "-s", path_str; "init with script and dry-run");

    assert!(output.status.success());
    assert!(!path.exists())
}

#[test]
fn test_init_bad_dirs() {
    let env = init();

    let path = PathBuf::from("/");
    let path_str = path.to_str().unwrap();
    let output = gourd!(env; "init", "-s", path_str);
    assert!(!output.status.success());

    let path = env
        .temp_dir
        .path()
        .join("nonexistent-parent")
        .join("parentless-path");
    let path_str = path.to_str().unwrap();
    let output = gourd!(env; "init", "-s", path_str);
    assert!(!output.status.success());

    let path = env.temp_dir.path().join("existing-path");
    fs::create_dir(&path).unwrap();
    let path_str = path.to_str().unwrap();
    let output = gourd!(env; "init", "-s", path_str);
    assert!(!output.status.success());
}

// todo: uncomment and fix

// #[test]
// fn test_init_interactive() {
//     let env = init();
//
//     // Create the child process with piped (blocking) stdio
//
//     let init_dir = env.temp_dir.path().join("init_test_interactive");
//     let gourd_command = env
//         .gourd_path
//         .to_str()
//         .expect("Could not get the path to gourd")
//         .to_owned()
//         + " init "
//         + init_dir.to_str().unwrap();
//
//     // This is needed to simulate a TTY.
//     // The inquire library doesn't work when it does not detect a terminal.
//     let mut gourd = fake_tty::command(&gourd_command, None)
//         .expect("Could not create a fake TTY")
//         .stdin(Stdio::piped())
//         .stdout(Stdio::piped())
//         .spawn()
//         .expect("Could not spawn gourd");
//
//     {
//         let stdin = gourd.stdin.as_mut().unwrap();
//
//         //  Specify custom output paths?
//         stdin.write_all(b"yes\n").unwrap();
//
//         // Path to experiments folder:
//         // Test of an absolute path (the folder shouldn't be created)
//         stdin.write_all(b"/arbitrary/abs/path\n").unwrap();
//         // Path to output folder:
//         // Test of an absolute path (the folder shouldn't be created)
//         stdin.write_all(b"output\n").unwrap();
//         // Path to metrics folder:
//         // Relative path (the folder should be created)
//         stdin.write_all(b"relative/metrics\n").unwrap();
//
//         // Include options for Slurm?
//         stdin.write_all(b"yes\n").unwrap();
//
//         // Slurm experiment name?
//         stdin.write_all(b"experiment_name\n").unwrap();
//         // Slurm array count limit?
//         stdin.write_all(b"1612\n").unwrap();
//         // Slurm array size limit?
//         stdin.write_all(b"50326\n").unwrap();
//         // Enter Slurm credentials now?
//         stdin.write_all(b"yes\n").unwrap();
//         // Slurm account to use?
//         stdin.write_all(b"account\n").unwrap();
//         // Slurm partition to use?
//         stdin.write_all(b"partition\n").unwrap();
//
//         // stdin is dropped here, output doesn't block
//     }
//
//     let output = gourd
//         .wait_with_output()
//         .expect("Could not wait for gourd init to finish");
//
//     if !output.status.success() {
//         stdout()
//             .write_all(&output.stdout)
//             .expect("Could not write failed result to stdout");
//         stderr()
//             .write_all(&output.stderr)
//             .expect("Could not write failed result to stderr");
//         panic!("Init command failed the interactive integration test");
//     }
//
//     // Check that the init was successful
//     assert!(init_dir.exists());
//     assert!(init_dir.join(".git").exists());
//
//     let gourd_toml = Config::from_file(&init_dir.join("gourd.toml"), &env.fs)
//         .expect("Could not read the gourd.toml created by gourd init");
//
//     let slurm_conf = gourd_toml
//         .slurm
//         .expect("`gourd init` did not create a SLURM config.");
//
//     assert_eq!("account", slurm_conf.account);
//     assert_eq!("partition", slurm_conf.partition);
//     assert_eq!(50326, slurm_conf.array_size_limit);
//     assert_eq!(1612, slurm_conf.array_count_limit);
//     assert_eq!("experiment_name", slurm_conf.experiment_name);
//     assert_eq!(PathBuf::from("output"), gourd_toml.output_path);
//     assert_eq!(PathBuf::from("relative/metrics"), gourd_toml.metrics_path);
//     assert_eq!(
//         PathBuf::from("/arbitrary/abs/path"),
//         gourd_toml.experiments_folder
//     );
//     assert!(
//         !gourd_toml.experiments_folder.is_relative() &&
// !gourd_toml.experiments_folder.exists()     );
//     assert!(gourd_toml.output_path.is_relative() &&
// init_dir.join(gourd_toml.output_path).is_dir());     assert!(
//         gourd_toml.metrics_path.is_relative() &&
// init_dir.join(gourd_toml.metrics_path).is_dir()     );
// }
