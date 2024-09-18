use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Result;
use gourd_lib::experiment::Experiment;
use gourd_lib::measurement::Measurement;

use crate::analyse::ColumnGenerator;
use crate::analyse::Table;
use crate::status::ExperimentStatus;
use crate::status::FsState;
use crate::status::Status;

/// Shorthand for creating a [`ColumnGenerator`] with a str header and a closure
/// body.
///
/// Note that the closure must be coercible to a function pointer.
fn create_column<X>(
    header: &str,
    body: fn(&Experiment, &X) -> Result<String>,
) -> ColumnGenerator<X> {
    ColumnGenerator {
        header: Some(header.to_string()),
        body,
        footer: |_, _| Ok(None),
    }
}

/// Same as [`create_column`], but with a footer closure.
fn create_column_full<X>(
    header: &str,
    body: fn(&Experiment, &X) -> Result<String>,
    footer: fn(&Experiment, &[X]) -> Result<Option<String>>,
) -> ColumnGenerator<X> {
    ColumnGenerator {
        header: Some(header.to_string()),
        body,
        footer,
    }
}

/// TODO: all metrics
pub fn metrics_generators() -> &'static BTreeMap<String, ColumnGenerator<(usize, Status)>> {
    /// TODO: documentation
    static ONCE: OnceLock<BTreeMap<String, ColumnGenerator<(usize, Status)>>> = OnceLock::new();
    ONCE.get_or_init(|| {
        let mut map = BTreeMap::new();
        map.insert(
            "program".to_string(),
            create_column("program", |exp: &Experiment, x: &(usize, Status)| {
                Ok(exp.get_program(&exp.runs[x.0])?.name.clone())
            }),
        );
        map.insert(
            "file".to_string(),
            create_column("input file", |exp, x: &(usize, Status)| {
                Ok(format!("{:?}", &exp.runs[x.0].input.file))
            }),
        );
        map.insert(
            "args".to_string(),
            create_column("input args", |exp, x: &(usize, Status)| {
                Ok(format!("{:?}", &exp.runs[x.0].input.args))
            }),
        );
        map.insert(
            "group".to_string(),
            create_column("input group", |exp: &Experiment, x: &(usize, Status)| {
                Ok(exp.runs[x.0].group.clone().unwrap_or("N/A".to_string()))
            }),
        );
        map.insert(
            "afterscript".to_string(),
            create_column("afterscript", |_, x| {
                Ok(x.1
                    .fs_status
                    .afterscript_completion
                    .clone()
                    .unwrap_or(Some("N/A".to_string()))
                    .unwrap_or("done, no label".to_string()))
            }),
        );
        map.insert(
            "slurm".to_string(),
            create_column("slurm", |_, x| {
                Ok(x.1
                    .slurm_status
                    .map_or("N/A".to_string(), |x| x.completion.to_string()))
            }),
        );
        map.insert(
            "fs_status".to_string(),
            create_column("file system status", |_, x| {
                Ok(format!("{:-}", x.1.fs_status.completion))
            }),
        );
        map.insert(
            "exit_code".to_string(),
            ColumnGenerator {
                header: Some("exit code".to_string()),
                body: |_, x: &(usize, Status)| {
                    Ok(match &x.1.fs_status.completion {
                        FsState::Completed(measurement) => {
                            format!("{:?}", measurement.exit_code)
                        }
                        _ => "N/A".to_string(),
                    })
                },
                footer: |_, _| Ok(None),
            },
        );
        map.insert(
            "wall_time".to_string(),
            create_column_full(
                "wall time",
                |_, x| {
                    Ok(match &x.1.fs_status.completion {
                        FsState::Completed(measurement) => format!("{:?}", measurement.wall_micros),
                        _ => "N/A".to_string(),
                    })
                },
                |_, runs| {
                    let (dt, n) = runs.iter().fold((0, 0), |(sum, count), run| {
                        match &run.1.fs_status.completion {
                            FsState::Completed(m) => (sum + m.wall_micros.as_nanos(), count + 1),
                            _ => (sum, count),
                        }
                    });

                    Ok(Some(format!("{:?}", Duration::from_nanos((dt / n) as u64))))
                },
            ),
        );
        map.insert(
            "user_time".to_string(),
            create_column_full(
                "user time",
                |_, x| {
                    Ok(match &x.1.fs_status.completion {
                        FsState::Completed(Measurement {
                            rusage: Some(r), ..
                        }) => format!("{:?}", r.utime),
                        _ => "N/A".to_string(),
                    })
                },
                |_, runs| {
                    let (dt, n) = runs.iter().fold((0, 0), |(sum, count), run| {
                        match &run.1.fs_status.completion {
                            FsState::Completed(Measurement {
                                rusage: Some(r), ..
                            }) => (sum + r.utime.as_nanos(), count + 1),
                            _ => (sum, count),
                        }
                    });

                    Ok(Some(format!("{:?}", Duration::from_nanos((dt / n) as u64))))
                },
            ),
        );
        map.insert(
            "system_time".to_string(),
            create_column_full(
                "system time",
                |_, x| {
                    Ok(match &x.1.fs_status.completion {
                        FsState::Completed(Measurement {
                            rusage: Some(r), ..
                        }) => format!("{:?}", r.stime),
                        _ => "N/A".to_string(),
                    })
                },
                |_, runs| {
                    let (dt, n) = runs.iter().fold((0, 0), |(sum, count), run| {
                        match &run.1.fs_status.completion {
                            FsState::Completed(Measurement {
                                rusage: Some(r), ..
                            }) => (sum + r.stime.as_nanos(), count + 1),
                            _ => (sum, count),
                        }
                    });

                    Ok(Some(format!("{:?}", Duration::from_nanos((dt / n) as u64))))
                },
            ),
        );

        // TODO: find a way to shorten these
        map.insert(
            "maxrss".to_string(),
            create_column_full(
                "max RSS",
                |_, x| {
                    Ok(match &x.1.fs_status.completion {
                        FsState::Completed(Measurement {
                            rusage: Some(r), ..
                        }) => format!("{:?}", r.maxrss),
                        _ => "N/A".to_string(),
                    })
                },
                |_, runs| {
                    let (total, n) = runs.iter().fold((0, 0), |(sum, count), run| {
                        match &run.1.fs_status.completion {
                            FsState::Completed(Measurement {
                                rusage: Some(r), ..
                            }) => (sum + r.maxrss, count + 1),
                            _ => (sum, count),
                        }
                    });

                    Ok(Some(format!("{:?}", (total / n))))
                },
            ),
        );
        map.insert(
            "minflt".to_string(),
            create_column_full(
                "soft page faults",
                |_, x| {
                    Ok(match &x.1.fs_status.completion {
                        FsState::Completed(Measurement {
                            rusage: Some(r), ..
                        }) => format!("{:?}", r.minflt),
                        _ => "N/A".to_string(),
                    })
                },
                |_, runs| {
                    let (total, n) = runs.iter().fold((0, 0), |(sum, count), run| {
                        match &run.1.fs_status.completion {
                            FsState::Completed(Measurement {
                                rusage: Some(r), ..
                            }) => (sum + r.minflt, count + 1),
                            _ => (sum, count),
                        }
                    });

                    Ok(Some(format!("{:?}", (total / n))))
                },
            ),
        );
        map.insert(
            "majflt".to_string(),
            create_column_full(
                "hard page faults",
                |_, x| {
                    Ok(match &x.1.fs_status.completion {
                        FsState::Completed(Measurement {
                            rusage: Some(r), ..
                        }) => format!("{:?}", r.majflt),
                        _ => "N/A".to_string(),
                    })
                },
                |_, runs| {
                    let (total, n) = runs.iter().fold((0, 0), |(sum, count), run| {
                        match &run.1.fs_status.completion {
                            FsState::Completed(Measurement {
                                rusage: Some(r), ..
                            }) => (sum + r.majflt, count + 1),
                            _ => (sum, count),
                        }
                    });

                    Ok(Some(format!("{:?}", (total / n))))
                },
            ),
        );
        map.insert(
            "nvcsw".to_string(),
            create_column_full(
                "voluntary context switches",
                |_, x| {
                    Ok(match &x.1.fs_status.completion {
                        FsState::Completed(Measurement {
                            rusage: Some(r), ..
                        }) => format!("{:?}", r.nvcsw),
                        _ => "N/A".to_string(),
                    })
                },
                |_, runs| {
                    let (total, n) = runs.iter().fold((0, 0), |(sum, count), run| {
                        match &run.1.fs_status.completion {
                            FsState::Completed(Measurement {
                                rusage: Some(r), ..
                            }) => (sum + r.nvcsw, count + 1),
                            _ => (sum, count),
                        }
                    });

                    Ok(Some(format!("{:?}", (total / n))))
                },
            ),
        );
        map.insert(
            "nivcsw".to_string(),
            create_column_full(
                "involuntary context switches",
                |_, x| {
                    Ok(match &x.1.fs_status.completion {
                        FsState::Completed(Measurement {
                            rusage: Some(r), ..
                        }) => format!("{:?}", r.nivcsw),
                        _ => "N/A".to_string(),
                    })
                },
                |_, runs| {
                    let (total, n) = runs.iter().fold((0, 0), |(sum, count), run| {
                        match &run.1.fs_status.completion {
                            FsState::Completed(Measurement {
                                rusage: Some(r), ..
                            }) => (sum + r.nivcsw, count + 1),
                            _ => (sum, count),
                        }
                    });

                    Ok(Some(format!("{:?}", (total / n))))
                },
            ),
        );

        map
    })
}

/// Generate a [`Table`] of metrics for this experiment.
///
/// Header:
/// ```text
/// | run id | program | input file | input args | afterscript | slurm? | file system status | exit code | wall time | user time | system time | max rss    | minor pf | major pf | voluntary cs | involuntary cs |
/// ```
pub fn metrics_table(experiment: &Experiment, statuses: &ExperimentStatus) -> Result<Table> {
    let header = [
        "program",
        "file",
        "args",
        "group",
        "afterscript",
        "slurm",
        "fs_status",
        "exit_code",
        "wall_time",
        "user_time",
        "system_time",
        "maxrss",
        "minflt",
        "majflt",
        "nvcsw",
        "nivcsw",
    ];

    let mut metrics_table = Table {
        columns: 1,
        header: Some(vec!["run id".into()]),
        body: statuses
            .keys()
            .map(|id| vec![format!("run {id}")])
            .collect(),
        footer: Some(vec!["average".into()]),
    };

    let generators = metrics_generators();

    for column_name in header {
        let status_tuples: Vec<(usize, Status)> = statuses.clone().into_iter().collect();
        let col = generators.get(column_name).unwrap();
        let column = col.generate(experiment, &status_tuples)?;
        metrics_table.append_column(column);
    }

    Ok(metrics_table)
}

/// Generate a [`Table`] of metrics for this experiment, with averages per input
/// group.
pub fn groups_table(_experiment: &Experiment, _statuses: &ExperimentStatus) -> Result<Vec<Table>> {
    // let mut grouped_runs: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    //
    // for (run_id, run_data) in experiment.runs.iter().enumerate() {
    //     if let Some(group) = &run_data.group {
    //         grouped_runs
    //             .entry(group.clone())
    //             .and_modify(|e| e.push(run_id))
    //             .or_insert(vec![run_id]);
    //     }
    // }
    //
    // let mut tables = vec![];
    // for (group, runs) in grouped_runs {
    //     // let mut groups_table = Table {
    //     //     header: Some([
    //     //         "group".into(),
    //     //         "run id".into(),
    //     //         "program".into(),
    //     //         "input file".into(),
    //     //         "input args".into(),
    //     //         "fs status".into(),
    //     //         "exit code".into(),
    //     //         "wall time".into(),
    //     //         "user time".into(),
    //     //         "system time".into(),
    //     //         "max rss".into(),
    //     //         "minor pf".into(),
    //     //         "major pf".into(),
    //     //         "voluntary cs".into(),
    //     //         "involuntary cs".into(),
    //     //     ]),
    //     //     body: vec![],
    //     //     footer: None,
    //     // };
    //
    //     let mut averages = [0f64; 8];
    //     let mut count = 0.0;
    //     for run_id in runs {
    //         let status = &statuses[&run_id];
    //         let mut record: [String; 15] = Default::default();
    //
    //         record[0] = group.clone();
    //         record[1] = run_id.to_string();
    //         record[2] = experiment
    //             .get_program(&experiment.runs[run_id])?
    //             .name
    //             .clone();
    //         record[3] = format!("{:?}", &experiment.runs[run_id].input.file);
    //         record[4] = format!("{:?}", &experiment.runs[run_id].input.args);
    //
    //         // let (fs_metrics, completed) = fs_metrics(status, &mut averages);
    //         // if completed {
    //         //     count += 1.0;
    //         // }
    //         // fs_metrics
    //         //     .iter()
    //         //     .enumerate()
    //         //     .for_each(|(i, x)| record[i + 5] = x.clone());
    //
    //         // groups_table.body.push(record);
    //     }
    //
    //     averages = averages.map(|x| x / count);
    //
    //     let mut footer: [String; 15] = Default::default();
    //     footer[6] = "Average:".into();
    //     footer[7] = format!("{:?}", Duration::from_nanos(averages[0] as u64));
    //     footer[8] = format!("{:?}", Duration::from_nanos(averages[1] as u64));
    //     footer[9] = format!("{:?}", Duration::from_nanos(averages[2] as u64));
    //     averages
    //         .iter()
    //         .skip(3)
    //         .enumerate()
    //         .for_each(|(i, a)| footer[i + 10] = format!("{a:.2}"));
    //
    //     // groups_table.footer = Some(footer);
    //
    //     tables.push(groups_table);
    // }
    todo!()
    // Ok(tables)
}
