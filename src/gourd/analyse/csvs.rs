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
// We cannot use a higher order function here because [`ColumnGenerator`] takes
// an fn() -> .. and not a closure (impl Fn()), for conciseness and readability
// there. the downside is that you can't use any environment variables in the
// closure, and that includes arguments passed to the higher order function.
// Macros are evaluated before compilation and thus circumvent this issue.
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

/// Get a [`ColumnGenerator`] for every possible column of [`CsvColumn`].
pub fn metrics_generators(col: CsvColumn) -> ColumnGenerator<(usize, Status)> {
    match col {
        CsvColumn::Program => create_column("program", |exp: &Experiment, x: &(usize, Status)| {
            Ok(exp.get_program(&exp.runs[x.0])?.name.clone())
        }),
        CsvColumn::File => create_column("input file", |exp, x: &(usize, Status)| {
            Ok(exp.runs[x.0]
                .input
                .file
                .as_ref()
                .map_or("None".to_string(), |p| format!("{p:?}")))
        }),
        CsvColumn::Args => create_column("input args", |exp, x: &(usize, Status)| {
            Ok(format!("{:?}", &exp.runs[x.0].input.args))
        }),
        CsvColumn::Group => create_column("group", |exp: &Experiment, x: &(usize, Status)| {
            Ok(exp.runs[x.0].group.clone().unwrap_or("N/A".to_string()))
        }),
        CsvColumn::Label => create_column("label", |_, x| {
            Ok(x.1
                .fs_status
                .afterscript_completion
                .clone()
                .unwrap_or(Some("N/A".to_string()))
                .unwrap_or("no label".to_string()))
        }),
        CsvColumn::Afterscript => create_column("afterscript", |exp, x| {
            exp.runs[x.0]
                .afterscript_output
                .as_ref()
                .map_or(Ok("N/A".to_string()), |p| Ok(p.trim().to_string()))
        }),
        CsvColumn::Slurm => create_column("slurm", |_, x| {
            Ok(x.1
                .slurm_status
                .map_or("N/A".to_string(), |x| x.completion.to_string()))
        }),
        CsvColumn::FsStatus => create_column("fs status", |_, x| {
            Ok(format!("{:-}", x.1.fs_status.completion))
        }),
        CsvColumn::ExitCode => ColumnGenerator {
            header: Some("exit".to_string()),
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
        CsvColumn::WallTime => create_column_full(
            "wall time",
            |_, x| {
                Ok(match &x.1.fs_status.completion {
                    FsState::Completed(measurement) => {
                        format!("{:.5}s", measurement.wall_micros.as_secs_f32())
                    }
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

                Ok(Some(format!(
                    "{:.5}s",
                    Duration::from_nanos((dt.checked_div(n).unwrap_or_default()) as u64)
                        .as_secs_f32()
                )))
            },
        ),
        CsvColumn::UserTime => create_column_full(
            "user time",
            |_, x| {
                Ok(match &x.1.fs_status.completion {
                    FsState::Completed(Measurement {
                        rusage: Some(r), ..
                    }) => format!("{:.5}s", r.utime.as_secs_f32()),
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

                Ok(Some(format!(
                    "{:.5}s",
                    Duration::from_nanos((dt.checked_div(n).unwrap_or_default()) as u64)
                        .as_secs_f32()
                )))
            },
        ),
        CsvColumn::SystemTime => create_column_full(
            "system time",
            |_, x| {
                Ok(match &x.1.fs_status.completion {
                    FsState::Completed(Measurement {
                        rusage: Some(r), ..
                    }) => format!("{:.5}s", r.stime.as_secs_f32()),
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

                Ok(Some(format!(
                    "{:.5}s",
                    Duration::from_nanos((dt.checked_div(n).unwrap_or_default()) as u64)
                        .as_secs_f32()
                )))
            },
        ),

        CsvColumn::MaxRSS => rusage_metrics!("max rss", |r: &RUsage| r.maxrss),
        CsvColumn::IxRSS => rusage_metrics!("shared mem size", |r: &RUsage| r.ixrss),
        CsvColumn::IdRSS => rusage_metrics!("unshared mem size", |r: &RUsage| r.idrss),
        CsvColumn::IsRSS => rusage_metrics!("unshared stack size", |r: &RUsage| r.isrss),
        CsvColumn::MinFlt => rusage_metrics!("soft page faults", |r: &RUsage| r.minflt),
        CsvColumn::MajFlt => rusage_metrics!("hard page faults", |r: &RUsage| r.majflt),
        CsvColumn::NSwap => rusage_metrics!("swaps", |r: &RUsage| r.nswap),
        CsvColumn::InBlock => rusage_metrics!("block input operations", |r: &RUsage| r.inblock),
        CsvColumn::OuBlock => rusage_metrics!("block output operations", |r: &RUsage| r.oublock),
        CsvColumn::MsgSent => rusage_metrics!("IPC messages sent", |r: &RUsage| r.msgsnd),
        CsvColumn::MsgRecv => rusage_metrics!("IPC messages received", |r: &RUsage| r.msgrcv),
        CsvColumn::NSignals => rusage_metrics!("signals received", |r: &RUsage| r.nsignals),
        CsvColumn::NVCsw => rusage_metrics!("voluntary context switches", |r: &RUsage| r.nvcsw),
        CsvColumn::NIvCsw => rusage_metrics!("involuntary context switches", |r: &RUsage| r.nivcsw),
    }
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
            .map(|(id, _)| vec![format!("{id}")])
            .collect(),
        footer: Some(vec!["average".into()]),
    };

    for column_name in header {
        let column = metrics_generators(column_name).generate(experiment, &status_tuples)?;
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
        CsvColumn::Slurm,
        CsvColumn::FsStatus,
        CsvColumn::WallTime,
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
