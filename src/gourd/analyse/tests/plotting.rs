use std::collections::BTreeMap;
use std::fs;
use std::time::Duration;

use gourd_lib::experiment::Environment;
use gourd_lib::experiment::InternalProgram;
use gourd_lib::experiment::Run;
use gourd_lib::experiment::RunInput;
use gourd_lib::measurement::Measurement;
use tempdir::TempDir;

use super::*;
use crate::cli::def::PlotType::Png;
use crate::cli::def::PlotType::Svg;
use crate::status::FileSystemBasedStatus;
use crate::status::FsState;
use crate::status::SlurmBasedStatus;
use crate::status::SlurmState;
use crate::status::Status;

#[test]
fn test_get_data_for_plot_exists() {
    let mut completions: BTreeMap<FieldRef, Vec<u128>> = BTreeMap::new();
    completions.insert("first".to_string(), vec![1, 2, 5]);
    completions.insert("second".to_string(), vec![1, 3]);

    let max_time = 5;
    let max_count = 3;

    let mut data: BTreeMap<FieldRef, Vec<(u128, u128)>> = BTreeMap::new();
    data.insert(
        "first".to_string(),
        vec![(0, 0), (1, 1), (1, 1), (2, 2), (4, 2), (5, 3), (5, 3)],
    );
    data.insert(
        "second".to_string(),
        vec![(0, 0), (1, 1), (2, 1), (3, 2), (5, 2)],
    );

    let res = get_data_for_plot(completions);
    assert_eq!((max_time, max_count, data), res);
}

#[test]
fn test_get_data_for_plot_not_exist() {
    let completions: BTreeMap<FieldRef, Vec<u128>> = BTreeMap::new();

    assert_eq!((0, 0, BTreeMap::new()), get_data_for_plot(completions));
}

#[test]
fn test_make_plot() {
    let tmp_dir = TempDir::new("testing").unwrap();
    let output_path = tmp_dir.path().join("plot.png");

    let mut data: BTreeMap<FieldRef, Vec<(u128, u128)>> = BTreeMap::new();
    data.insert(
        "first".to_string(),
        vec![(0, 0), (1, 1), (2, 2), (3, 2), (4, 2), (5, 3)],
    );
    data.insert(
        "second".to_string(),
        vec![(0, 0), (1, 1), (2, 1), (3, 2), (4, 2), (5, 2)],
    );

    assert!(make_plot((5, 3, data), BitMapBackend::new(&output_path, (300, 300))).is_ok());
}

#[test]
fn test_analysis_png_plot_success() {
    let tmp_dir = TempDir::new("testing").unwrap();
    let mut statuses = BTreeMap::new();
    let status_with_rusage = Status {
        slurm_file_text: None,
        fs_status: FileSystemBasedStatus {
            completion: FsState::Completed(Measurement {
                wall_micros: Duration::from_nanos(0),
                exit_code: 0,
                rusage: Some(crate::analyse::tests::TEST_RUSAGE),
            }),
            afterscript_completion: None,
        },
        slurm_status: Some(SlurmBasedStatus {
            completion: SlurmState::Success,
            exit_code_program: 0,
            exit_code_slurm: 0,
        }),
    };
    let mut status_no_rusage = status_with_rusage.clone();
    status_no_rusage.fs_status.completion = FsState::Completed(Measurement {
        wall_micros: Duration::from_nanos(0),
        exit_code: 0,
        rusage: None,
    });
    statuses.insert(
        0,
        Status {
            fs_status: FileSystemBasedStatus {
                completion: crate::status::FsState::Pending,
                afterscript_completion: Some(Some(String::from("lol-label"))),
            },
            slurm_status: None,
            slurm_file_text: None,
        },
    );
    statuses.insert(1, status_no_rusage);
    statuses.insert(2, status_with_rusage.clone());
    statuses.insert(3, status_with_rusage);
    let run = Run {
        program: 0,
        input: RunInput {
            file: None,
            args: Vec::new(),
        },
        err_path: Default::default(),
        output_path: Default::default(),
        metrics_path: Default::default(),
        work_dir: Default::default(),
        slurm_id: None,
        afterscript_output_path: None,
        rerun: None,
        generated_from_input: None,
        parent: None,
        limits: Default::default(),
        group: None,
    };
    let experiment = Experiment {
        runs: vec![run.clone(), run.clone(), run.clone(), run],
        resource_limits: None,
        creation_time: Default::default(),
        home: Default::default(),
        wrapper: "".to_string(),
        inputs: Default::default(),
        programs: vec![InternalProgram::default()],
        output_folder: Default::default(),
        metrics_folder: Default::default(),
        seq: 0,
        env: Environment::Local,
        labels: Default::default(),
        afterscript_output_folder: Default::default(),
        slurm: None,
        chunks: vec![],
        groups: vec![],
    };

    let png_output_path = tmp_dir.path().join("analysis.png");
    analysis_plot(&png_output_path, statuses.clone(), &experiment, Png).unwrap();

    assert!(&png_output_path.exists());
    assert!(fs::read(&png_output_path).is_ok_and(|r| !r.is_empty()));

    let svg_output_path = tmp_dir.path().join("analysis.svg");
    analysis_plot(&svg_output_path, statuses, &experiment, Svg).unwrap();

    assert!(&svg_output_path.exists());
    assert!(fs::read(&svg_output_path).is_ok_and(|r| !r.is_empty()));
}
