use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anstyle::AnsiColor;
use anstyle::Color;
use anstyle::Style;
use chrono::Local;
use gourd_lib::config::Config;
use gourd_lib::config::UserInput;
use gourd_lib::config::UserProgram;
use gourd_lib::constants::style_from_fg;
use gourd_lib::experiment::Environment;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::FieldRef;
use gourd_lib::file_system::FileSystemInteractor;
use tempdir::TempDir;

use crate::experiments::ExperimentExt;

/// a file system interactor that *will* touch files
pub const REAL_FS: FileSystemInteractor = FileSystemInteractor { dry_run: false };

/// compile the rust file provided into a binary, and place it in its own
/// tempdir
pub fn get_compiled_example(contents: &str, extra_args: Option<Vec<&str>>) -> (PathBuf, PathBuf) {
    let tmp = TempDir::new("match").unwrap().into_path();

    let source = tmp.join("prog.rs");
    let out = tmp.join("prog");

    fs::write(&source, contents).unwrap();

    let mut cmd = Command::new("rustc");
    cmd.arg(source.canonicalize().unwrap()).arg("-o").arg(&out);
    if let Some(extra_args) = extra_args {
        cmd.args(extra_args);
    }
    cmd.spawn().unwrap().wait().unwrap();

    (out, tmp)
}

/// a template experiment
pub fn create_sample_experiment(
    prog: BTreeMap<FieldRef, UserProgram>,
    inputs: BTreeMap<FieldRef, UserInput>,
) -> (Experiment, Config) {
    let conf = Config {
        output_path: TempDir::new("output").unwrap().into_path(),
        metrics_path: TempDir::new("metrics").unwrap().into_path(),
        experiments_folder: TempDir::new("experiments").unwrap().into_path(),
        wrapper: "".to_string(),
        programs: prog,
        inputs,
        input_schema: None,
        parameters: None,
        slurm: None,
        resource_limits: None,
        labels: Some(BTreeMap::new()),
    };

    (
        Experiment::from_config(&conf, Local::now(), Environment::Local, &REAL_FS).unwrap(),
        conf,
    )
}

#[test]
fn test_style() {
    assert_eq!(
        style_from_fg(AnsiColor::Red),
        Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)))
    );
}
