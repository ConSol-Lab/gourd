use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Result;
use gourd_lib::experiment::Experiment;
use gourd_lib::measurement::Measurement;
use gourd_lib::measurement::RUsage;

use crate::analyse::ColumnGenerator;
use crate::analyse::Table;
use crate::cli::def::CsvColumn;
use crate::cli::def::CsvFormatting;
use crate::cli::def::GroupBy;
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

/// Shorthand to create a column generator for a metric that is derived from the
/// `rusage`
macro_rules! rusage_metrics {
    ($name:expr, $field:expr) => {
        create_column_full(
            $name,
            |_, x| {
                Ok(match &x.1.fs_status.completion {
                    FsState::Completed(Measurement {
                        rusage: Some(r), ..
                    }) => format!("{:?}", $field(r)),
                    _ => "N/A".to_string(),
                })
            },
            |_, runs| {
                let (total, n) = runs.iter().fold((0, 0), |(sum, count), run| {
                    match &run.1.fs_status.completion {
                        FsState::Completed(Measurement {
                            rusage: Some(r), ..
                        }) => (sum + $field(r), count + 1),
                        _ => (sum, count),
                    }
                });

                Ok(Some(format!("{:.2}", ((total as f64) / (n as f64)))))
            },
        )
    };
}

/// Create a map of column generators for the metrics that can be included in
/// CSV analysis
pub fn metrics_generators() -> &'static BTreeMap<CsvColumn, ColumnGenerator<(usize, Status)>> {
    /// A `OnceLock` to ensure that the metrics generators are only created once
    /// (and not for every table in case of grouping).
    static ONCE: OnceLock<BTreeMap<CsvColumn, ColumnGenerator<(usize, Status)>>> = OnceLock::new();
    ONCE.get_or_init(|| {
        let mut map = BTreeMap::new();
        map.insert(
            CsvColumn::Program,
            create_column("program", |exp: &Experiment, x: &(usize, Status)| {
                Ok(exp.get_program(&exp.runs[x.0])?.name.clone())
            }),
        );
        map.insert(
            CsvColumn::File,
            create_column("input file", |exp, x: &(usize, Status)| {
                Ok(format!("{:?}", &exp.runs[x.0].input.file))
            }),
        );
        map.insert(
            CsvColumn::Args,
            create_column("input args", |exp, x: &(usize, Status)| {
                Ok(format!("{:?}", &exp.runs[x.0].input.args))
            }),
        );
        map.insert(
            CsvColumn::Group,
            create_column("input group", |exp: &Experiment, x: &(usize, Status)| {
                Ok(exp.runs[x.0].group.clone().unwrap_or("N/A".to_string()))
            }),
        );
        map.insert(
            CsvColumn::Afterscript,
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
            CsvColumn::Slurm,
            create_column("slurm", |_, x| {
                Ok(x.1
                    .slurm_status
                    .map_or("N/A".to_string(), |x| x.completion.to_string()))
            }),
        );
        map.insert(
            CsvColumn::FsStatus,
            create_column("file system status", |_, x| {
                Ok(format!("{:-}", x.1.fs_status.completion))
            }),
        );
        map.insert(
            CsvColumn::ExitCode,
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
            CsvColumn::WallTime,
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
            CsvColumn::UserTime,
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
            CsvColumn::SystemTime,
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

        map.insert(
            CsvColumn::MaxRSS,
            rusage_metrics!("max rss", |r: &RUsage| r.maxrss),
        );
        map.insert(
            CsvColumn::IxRSS,
            rusage_metrics!("shared mem size", |r: &RUsage| r.ixrss),
        );
        map.insert(
            CsvColumn::IdRSS,
            rusage_metrics!("unshared mem size", |r: &RUsage| r.idrss),
        );
        map.insert(
            CsvColumn::IsRSS,
            rusage_metrics!("unshared stack size", |r: &RUsage| r.isrss),
        );
        map.insert(
            CsvColumn::MinFlt,
            rusage_metrics!("soft page faults", |r: &RUsage| r.minflt),
        );
        map.insert(
            CsvColumn::MajFlt,
            rusage_metrics!("hard page faults", |r: &RUsage| r.majflt),
        );
        map.insert(
            CsvColumn::NSwap,
            rusage_metrics!("swaps", |r: &RUsage| r.nswap),
        );
        map.insert(
            CsvColumn::InBlock,
            rusage_metrics!("block input operations", |r: &RUsage| r.inblock),
        );
        map.insert(
            CsvColumn::OuBlock,
            rusage_metrics!("block output operations", |r: &RUsage| r.oublock),
        );
        map.insert(
            CsvColumn::MsgSent,
            rusage_metrics!("IPC messages sent", |r: &RUsage| r.msgsnd),
        );
        map.insert(
            CsvColumn::MsgRecv,
            rusage_metrics!("IPC messages received", |r: &RUsage| r.msgrcv),
        );
        map.insert(
            CsvColumn::NSignals,
            rusage_metrics!("signals received", |r: &RUsage| r.nsignals),
        );
        map.insert(
            CsvColumn::NVCsw,
            rusage_metrics!("voluntary context switches", |r: &RUsage| r.nvcsw),
        );
        map.insert(
            CsvColumn::NIvCsw,
            rusage_metrics!("involuntary context switches", |r: &RUsage| r.nivcsw),
        );

        map
    })
}

/// Generate a [`Table`] of metrics for this experiment.
/// TODO: better documentation
pub fn metrics_table(
    experiment: &Experiment,
    header: Vec<CsvColumn>,
    status_tuples: Vec<(usize, Status)>,
) -> Result<Table> {
    let mut metrics_table = Table {
        columns: 1,
        header: Some(vec!["run id".into()]),
        body: status_tuples
            .iter()
            .map(|(id, _)| vec![format!("run {id}")])
            .collect(),
        footer: Some(vec!["average".into()]),
    };

    let generators = metrics_generators();

    for column_name in header {
        let col = generators[&column_name].clone();
        let column = col.generate(experiment, &status_tuples)?;
        metrics_table.append_column(column);
    }

    Ok(metrics_table)
}

/// Generate a vector of [`Table`]s from an experiment and its status.
pub fn tables_from_command(
    experiment: &Experiment,
    statuses: &ExperimentStatus,
    fmt: CsvFormatting,
) -> Result<Vec<Table>> {
    let header = fmt.format.unwrap_or(vec![
        CsvColumn::Program,
        CsvColumn::File,
        CsvColumn::Args,
        CsvColumn::Group,
        CsvColumn::Afterscript,
        CsvColumn::Slurm,
        CsvColumn::FsStatus,
        CsvColumn::ExitCode,
        CsvColumn::WallTime,
        CsvColumn::UserTime,
        CsvColumn::SystemTime,
    ]);

    let mut groups: Vec<Vec<(usize, Status)>> = vec![statuses.clone().into_iter().collect()];

    for condition in fmt.group {
        let mut temp = vec![];
        for g in groups {
            match condition {
                GroupBy::Group => {
                    g.chunk_by(|(a_id, _), (b_id, _)| {
                        experiment.runs[*a_id].group == experiment.runs[*b_id].group
                    })
                    .for_each(|x| temp.push(x.to_vec()));
                }
                GroupBy::Input => {
                    g.chunk_by(|(a_id, _), (b_id, _)| {
                        experiment.runs[*a_id].input == experiment.runs[*b_id].input
                    })
                    .for_each(|x| temp.push(x.to_vec()));
                }
                GroupBy::Program => {
                    g.chunk_by(|(a_id, _), (b_id, _)| {
                        experiment.runs[*a_id].program == experiment.runs[*b_id].program
                    })
                    .for_each(|x| temp.push(x.to_vec()));
                }
            }
        }
        groups = temp;
    }

    groups
        .into_iter()
        .map(|runs| metrics_table(experiment, header.clone(), runs))
        .collect()
}
