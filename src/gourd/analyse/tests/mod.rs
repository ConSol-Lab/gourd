use std::collections::BTreeMap;
use std::default::Default;
use std::fs;
use std::time::Duration;

use csv::Reader;
use csv::StringRecord;
use gourd_lib::experiment::Environment;
use gourd_lib::experiment::InternalProgram;
use gourd_lib::experiment::Run;
use gourd_lib::experiment::RunInput;
use gourd_lib::measurement::Measurement;
use tempdir::TempDir;

use super::*;
use crate::status::SlurmState;

static TEST_RUSAGE: RUsage = RUsage {
    utime: Duration::from_micros(2137),
    stime: Duration::from_micros(2137),
    maxrss: 2137,
    ixrss: 2137,
    idrss: 2137,
    isrss: 2137,
    minflt: 2137,
    majflt: 2137,
    nswap: 2137,
    inblock: 2137,
    oublock: 2137,
    msgsnd: 2137,
    msgrcv: 2137,
    nsignals: 2137,
    nvcsw: 2137,
    nivcsw: 2137,
};

#[test]
fn test_analysis_csv_unwritable() {
    let tmp_dir = TempDir::new("testing").unwrap();

    let output_path = tmp_dir.path().join("analysis.csv");

    // By creating a directory, the path becomes unwritable
    let _ = fs::create_dir(&output_path);

    assert!(analysis_csv(&output_path, BTreeMap::new()).is_err());
}

#[test]
fn test_analysis_csv_success() {
    let tmp_dir = TempDir::new("testing").unwrap();

    let output_path = tmp_dir.path().join("analysis.csv");
    let mut statuses = BTreeMap::new();
    statuses.insert(
        0,
        Status {
            slurm_file_text: None,

            fs_status: FileSystemBasedStatus {
                completion: crate::status::FsState::Pending,
                afterscript_completion: Some(Some(String::from("lol-label"))),
            },
            slurm_status: None,
        },
    );
    statuses.insert(
        1,
        Status {
            slurm_file_text: None,

            fs_status: FileSystemBasedStatus {
                completion: FsState::Completed(Measurement {
                    wall_micros: Duration::from_nanos(0),
                    exit_code: 0,
                    rusage: None,
                }),
                afterscript_completion: None,
            },
            slurm_status: Some(SlurmBasedStatus {
                completion: SlurmState::Success,
                exit_code_program: 0,
                exit_code_slurm: 0,
            }),
        },
    );

    analysis_csv(&output_path, statuses).unwrap();

    let mut reader = Reader::from_path(output_path).unwrap();

    let res1 = reader.records().next();
    let ans1 = StringRecord::from(vec![
        "0",
        "pending",
        "...",
        "...",
        "...",
        "lol-label",
        "...",
    ]);
    assert_eq!(res1.unwrap().unwrap(), ans1);

    let res2 = reader.records().next();
    let ans2 = StringRecord::from(vec![
        "1",
        "completed",
        "0ns",
        "0",
        "none",
        "no afterscript",
        "Success",
    ]);
    assert_eq!(res2.unwrap().unwrap(), ans2);

    assert!(tmp_dir.close().is_ok());
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
                rusage: Some(TEST_RUSAGE),
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
            arguments: Vec::new(),
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
    analysis_plot(&png_output_path, statuses.clone(), experiment.clone(), true).unwrap();

    assert!(&png_output_path.exists());
    assert!(fs::read(&png_output_path).is_ok_and(|r| !r.is_empty()));

    let svg_output_path = tmp_dir.path().join("analysis.svg");
    analysis_plot(&svg_output_path, statuses, experiment, false).unwrap();

    assert!(&svg_output_path.exists());
    assert!(fs::read(&svg_output_path).is_ok_and(|r| !r.is_empty()));
}

#[test]
fn test_analysis_csv_wrong_path() {
    let tmp_dir = TempDir::new("testing").unwrap();

    let output_path = tmp_dir.path().join("");
    let statuses = BTreeMap::new();

    assert!(analysis_csv(&output_path, statuses).is_err());
    assert!(tmp_dir.close().is_ok());
}

#[test]
fn test_get_fs_status_info_pending() {
    let fs_status = FileSystemBasedStatus {
        completion: FsState::Pending,
        afterscript_completion: None,
    };
    let res = get_fs_status_info(0, &fs_status);
    assert_eq!(res, vec!["0", "pending", "...", "...", "..."]);
}

#[test]
fn test_get_fs_status_info_running() {
    let fs_status = FileSystemBasedStatus {
        completion: FsState::Running,
        afterscript_completion: None,
    };
    let res = get_fs_status_info(0, &fs_status);
    assert_eq!(res, vec!["0", "running", "...", "...", "..."]);
}

#[test]
fn test_get_fs_status_info_completed() {
    let fs_status = FileSystemBasedStatus {
        completion: FsState::Completed(Measurement {
            wall_micros: Duration::from_nanos(20),
            exit_code: 0,
            rusage: None,
        }),
        afterscript_completion: None,
    };
    let res = get_fs_status_info(0, &fs_status);
    assert_eq!(res, vec!["0", "completed", "20ns", "0", "none"]);
}

#[test]
fn test_format_rusage() {
    let res = format_rusage(Some(TEST_RUSAGE));
    let ans = "RUsage {\n    utime: 2.137ms,\n    stime: 2.137ms,\n    maxrss: 2137,\n    ixrss: 2137,\n    idrss: 2137,\n    isrss: 2137,\n    minflt: 2137,\n    majflt: 2137,\n    nswap: 2137,\n    inblock: 2137,\n    oublock: 2137,\n    msgsnd: 2137,\n    msgrcv: 2137,\n    nsignals: 2137,\n    nvcsw: 2137,\n    nivcsw: 2137,\n}";
    assert_eq!(res, ans);
}

#[test]
fn test_get_slurm_status_info() {
    let slurm = SlurmBasedStatus {
        completion: SlurmState::NodeFail,
        exit_code_program: 42,
        exit_code_slurm: 69,
    };

    assert_eq!(
        get_slurm_status_info(&Some(slurm)),
        vec![String::from("NodeFail")]
    );
    assert_eq!(get_slurm_status_info(&None), vec![String::from("...")]);
}

#[test]
fn test_get_afterscript_output_info() {
    let afterscript = Some(Some(String::from("lol-label")));

    assert_eq!(
        get_afterscript_output_info(&afterscript),
        vec![String::from("lol-label")]
    );
    assert_eq!(
        get_afterscript_output_info(&Some(None)),
        vec![String::from("done, no label")]
    );
    assert_eq!(
        get_afterscript_output_info(&None),
        vec![String::from("no afterscript")]
    );
}

#[test]
fn test_get_completion_time() {
    let state = FsState::Completed(Measurement {
        wall_micros: Duration::from_nanos(20),
        exit_code: 0,
        rusage: Some(TEST_RUSAGE),
    });
    let res = get_completion_time(state).unwrap();

    assert_eq!(Duration::from_micros(2137), res);
}

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
