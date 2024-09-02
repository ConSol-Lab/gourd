//! # Integration tests for the command line of `gourd`.
//! The tests in this module are exclusive to the user interactions AND local
//! execution functionality of `gourd`. Any user flows in any commands that
//! require presence of `Slurm` are not to be tested here, as they are not meant
//! for the CI pipeline.
//!
//! ## Test Plan
//!
//! + [x] Test the `gourd --version` command.
//! + [x] Test the `gourd run` command.
//! + [x] Test the `gourd init` command.
//! + [x] Test the `gourd status` command.
//! + [x] Test the `gourd rerun` command.
//! + [x] Test the `gourd analyse` command.
//!
//! ## Test Strategy
//! We have one environment, in code as `TestEnv`, in practice a working
//! directory, that is used by all the tests.
//! This environment contains:
//! - the (shared) compiled example binaries
//! - the (shared) input files for every example
//! - the gourd.toml configurations for each test
//! - the gourd output folders

mod afterscript;
mod analyse;
mod example;
mod init_example;
mod init_interactive;
mod rerun;
mod run;
mod version;
mod workflow;

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::Output;
use std::string::String;

use anyhow::anyhow;
use anyhow::Result;
use gourd_lib::config::Config;
use gourd_lib::config::UserProgram;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::FieldRef;
use gourd_lib::file_system::FileSystemInteractor;
use tempdir::TempDir;

/// The testing environment passed to individual #[test](s)
#[allow(dead_code)]
#[derive(Debug)]
struct TestEnv {
    gourd_path: PathBuf,
    wrapper_path: PathBuf,
    temp_dir: TempDir,
    programs: BTreeMap<FieldRef, UserProgram>,
    input_files: BTreeMap<String, PathBuf>,
    fs: FileSystemInteractor,
}

/// take a map and keep only the keys that are in the given list
fn keep<K: PartialEq, V, M: FromIterator<(K, V)> + IntoIterator<Item = (K, V)> + Clone>(
    map: &M,
    keys: &[K],
) -> M {
    map.clone()
        .into_iter()
        .filter(|(k, _)| keys.contains(k))
        .collect()
}

/// Disables RUST_BACKTRACE, executes `gourd` with arguments and appropriate
/// error handling.
///
/// The first expression must evaluate to a TestEnv struct.
/// The second expression set (after the semicolon) are string arguments to pass
/// to `gourd` The third expression is optional and provides error context.
#[macro_export]
macro_rules! gourd {
    ($env:expr; $($arg:expr),*) => {
        {
            let backtrace = std::env::var("RUST_BACKTRACE").unwrap_or("0".to_string());
            std::env::set_var("RUST_BACKTRACE", "0");
            let out = std::process::Command::new($env.gourd_path.clone()).args(&[$($arg),*]).output().unwrap();
            std::env::set_var("RUST_BACKTRACE", backtrace);
            out
        }
    };
    ($env:expr; $($arg:expr),*; $msg:expr) => {
        {
            let backtrace = std::env::var("RUST_BACKTRACE").unwrap_or("0".to_string());
            std::env::set_var("RUST_BACKTRACE", "0");
            let out = std::process::Command::new($env.gourd_path.clone()).args(&[$($arg),*]).output().unwrap();
            std::env::set_var("RUST_BACKTRACE", backtrace);
            if !out.status.success() {
                panic!("gourd {} failed: {}", $msg, String::from_utf8(out.stderr).unwrap());
            } else {
                out
            }
        }
    };
}

fn save_gourd_toml(conf: &Config, temp_dir: &TempDir) -> PathBuf {
    let conf_path = temp_dir.path().join("gourd.toml");
    let conf_str = toml::to_string(&conf).unwrap();
    std::fs::write(&conf_path, conf_str).unwrap();
    conf_path
}

fn compile_example(dir: &PathBuf, contents: &str, extra_args: Option<Vec<&str>>) -> PathBuf {
    // we create a new temp dir so that we don't need to specify program name
    // additionally, checking that it's not taken, having to randomise name,
    // or deal with file collisions, ever.
    let tmp = TempDir::new_in(dir, "example_program").unwrap().into_path();

    let source = tmp.join("prog.rs");
    let out = tmp.join("prog");

    std::fs::write(&source, contents).unwrap();

    let mut cmd = std::process::Command::new("rustc");
    cmd.arg(source.canonicalize().unwrap()).arg("-o").arg(&out);
    if let Some(extra_args) = extra_args {
        cmd.args(extra_args);
    }
    cmd.spawn().unwrap().wait().unwrap();

    out
}

fn new_program(
    prog: &mut BTreeMap<FieldRef, UserProgram>,
    name: &str,
    dir: &PathBuf,
    contents: &str,
    extra_args: Vec<&str>,
    post: Option<&str>,
) {
    prog.insert(
        name.to_string(),
        UserProgram {
            binary: Some(compile_example(dir, contents, None)),
            fetch: None,
            git: None,
            arguments: extra_args.iter().map(|s| s.to_string()).collect(),
            afterscript: None,
            next: post.map(|p| vec![p.to_string()]).unwrap_or_default(),
            resource_limits: None,
        },
    );
}

fn new_input(inputs: &mut BTreeMap<String, PathBuf>, name: &str, dir: &Path, contents: &str) {
    let path = dir.join(name);
    std::fs::write(dir.join(name), contents).unwrap();
    inputs.insert(name.to_string(), path);
}

fn init() -> TestEnv {
    // 1. find gourd cli executable
    let gourd_path = PathBuf::from(env!("CARGO_BIN_EXE_gourd"));
    assert!(
        &gourd_path.exists(),
        "\nTest setup couldn't find the gourd executable.
    Please ensure that both `gourd` and `gourd_wrapper` are built before running integration tests.
    [Expected to find the wrapper at: {:?}]\n",
        gourd_path
    );

    // 2. find gourd_wrapper executable
    let wrapper_path = PathBuf::from(env!("CARGO_BIN_EXE_gourd_wrapper"));
    assert!(
        Path::new(&wrapper_path).exists(),
        "\nTest setup couldn't find the wrapper executable.
    Please ensure that both `gourd` and `gourd_wrapper` are built before running integration tests.
    [Expected to find the wrapper at: {:?}]\n",
        wrapper_path
    );

    // Create a temporary directory to run experiments in. CARGO_TARGET_TMPDIR means
    // you can debug by looking in the ./target folder instead of wherever
    // /private/var/ tempdir decided to dump
    let temp_dir = TempDir::new_in(env!("CARGO_TARGET_TMPDIR"), "resources").unwrap();
    let p = temp_dir.path().to_path_buf();
    // initialise the programs and input files available in the testing environment.
    let mut programs = BTreeMap::default();
    let mut input_files = BTreeMap::new();

    // compiled examples
    new_program(
        &mut programs,
        "fibonacci",
        &p,
        include_str!("test_resources/fibonacci.rs"),
        vec![],
        None,
    );

    new_program(
        &mut programs,
        "slow_fib",
        &p,
        include_str!("test_resources/slow_fib.rs"),
        vec![],
        None,
    );

    new_program(
        &mut programs,
        "fast_fib",
        &p,
        include_str!("test_resources/fast_fib.rs"),
        vec![],
        Some("fast_fast_fib"),
    );

    new_program(
        &mut programs,
        "hello",
        &p,
        include_str!("test_resources/hello.rs"),
        vec!["hello"],
        None,
    );

    new_program(
        &mut programs,
        "fast_fast_fib",
        &p,
        include_str!("test_resources/fast_fib.rs"),
        vec!["-f"],
        None,
    );

    // construct some inputs
    new_input(&mut input_files, "input_ten", &p, "10");
    new_input(&mut input_files, "input_forty_two", &p, "42");
    new_input(&mut input_files, "input_hello", &p, "you");

    // finally, construct the test environment
    TestEnv {
        gourd_path,
        wrapper_path,
        temp_dir,
        programs,
        input_files,
        fs: FileSystemInteractor { dry_run: false },
    }
}

// A convenience macro that creates a configuration for integration testing.
//
// First expression: the environment (created using init())
// Second expression (list): a list of program IDs, a subset of integration
// testing example programs Third expression (list): a list of tuples of the
// form (input_id, input)
#[macro_export]
macro_rules! config {
    ($env:expr; $($prog:expr),*; $($inp:expr),*) => {
        {
            gourd_lib::config::Config {
                output_path: $env.temp_dir.path().join("out"),
                metrics_path: $env.temp_dir.path().join("metrics"),
                experiments_folder: $env.temp_dir.path().join("experiments"),
                programs: $crate::keep(&$env.programs.clone(), &[$($prog.to_string()),*]),
                inputs: std::collections::BTreeMap::<String, gourd_lib::config::UserInput>::from([$($inp),*]).into(),
                parameters: None,
                wrapper: $env.wrapper_path.to_str().unwrap().to_string(),
                input_schema: None,
                slurm: None,
                resource_limits: None,
                labels: None,
                warn_on_label_overlap: false,
            }
        }
    };

    ($env:expr; $($prog:expr),*; $($inp:expr),*; $label:expr) => {
        {
            gourd_lib::config::Config {
                output_path: $env.temp_dir.path().join("out"),
                metrics_path: $env.temp_dir.path().join("metrics"),
                experiments_folder: $env.temp_dir.path().join("experiments"),
                programs: $crate::keep(&$env.programs.clone(), &[$($prog.to_string()),*]),
                inputs: std::collections::BTreeMap::<String, gourd_lib::config::UserInput>::from([$($inp),*]).into(),
                parameters: None,
                wrapper: $env.wrapper_path.to_str().unwrap().to_string(),
                input_schema: None,
                slurm: None,
                resource_limits: None,
                labels: $label,
                warn_on_label_overlap: false,
            }
        }
    };
}

fn read_experiment_from_stdout(output: &Output) -> Result<Experiment> {
    let exp = std::fs::read_to_string(
        String::from_utf8(output.stdout.clone())?
            .split('\n')
            .nth_back(1)
            .ok_or(anyhow!("run didn't print experiment location"))?,
    )?;
    Ok(toml::from_str(&exp)?)
}
