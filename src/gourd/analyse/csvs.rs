use std::time::Duration;

use anyhow::Result;
use gourd_lib::experiment::Experiment;

use crate::analyse::Table;
use crate::status::ExperimentStatus;
use crate::status::FsState;

/// Generate a [`Table`] of metrics for this experiment.
///
/// Header:
/// ```text
/// | run id | program | input file | input args | afterscript | slurm? | file system status | exit code | wall time | user time | system time | max rss    | minor pf | major pf | voluntary cs | involuntary cs |
/// ```
pub fn metrics_table(
    experiment: &Experiment,
    statuses: &ExperimentStatus,
) -> Result<Table<String, 16>> {
    let header = [
        "run id".into(),
        "program".into(),
        "input file".into(),
        "input args".into(),
        "afterscript".into(),
        "slurm?".into(),
        "file system status".into(),
        "exit code".into(),
        "wall time".into(),
        "user time".into(),
        "system time".into(),
        "max rss".into(),
        "minor pf".into(),
        "major pf".into(),
        "voluntary cs".into(),
        "involuntary cs".into(),
    ];

    let mut metrics_table = Table {
        header: Some(header),
        body: vec![],
        footer: None,
    };

    let mut averages = [0f64; 8];
    let mut count = 0.0;
    for (id, status) in statuses {
        let mut record: [String; 16] = Default::default();

        record[0] = id.to_string();
        record[1] = experiment.get_program(&experiment.runs[*id])?.name.clone();
        record[2] = format!("{:?}", &experiment.runs[*id].input.file);
        record[3] = format!("{:?}", &experiment.runs[*id].input.args);
        record[4] = status
            .fs_status
            .afterscript_completion
            .clone()
            .unwrap_or(Some("N/A".to_string()))
            .unwrap_or("done, no label".to_string());
        record[5] = status
            .slurm_status
            .map_or("N/A".to_string(), |x| x.completion.to_string());

        match &status.fs_status.completion {
            FsState::Pending => {
                let mut x: [String; 10] = Default::default();
                x[0] = "pending".into();
                x
            }
            FsState::Running => {
                let mut x: [String; 10] = Default::default();
                x[0] = "running".into();
                x
            }
            FsState::Completed(measurement) => {
                count += 1.0;
                averages[0] += measurement.wall_micros.as_nanos() as f64;
                if let Some(r) = measurement.rusage {
                    averages[1] += r.utime.as_nanos() as f64;
                    averages[2] += r.stime.as_nanos() as f64;
                    averages[3] += r.maxrss as f64;
                    averages[4] += r.minflt as f64;
                    averages[5] += r.majflt as f64;
                    averages[6] += r.nvcsw as f64;
                    averages[7] += r.nivcsw as f64;
                    [
                        "completed".into(),
                        format!("{:?}", measurement.exit_code),
                        format!("{:?}", measurement.wall_micros),
                        format!("{:?}", r.utime),
                        format!("{:?}", r.stime),
                        r.maxrss.to_string(),
                        r.minflt.to_string(),
                        r.majflt.to_string(),
                        r.nvcsw.to_string(),
                        r.nivcsw.to_string(),
                    ]
                } else {
                    let mut x: [String; 10] = Default::default();
                    x[0] = "completed".into();
                    x[1] = format!("{:?}", measurement.exit_code);
                    x[2] = format!("{:?}", measurement.wall_micros);
                    x
                }
            }
        }
        .iter()
        .enumerate()
        .for_each(|(i, x)| record[i + 6] = x.clone());

        metrics_table.body.push(record);
    }

    averages = averages.map(|x| x / count);

    let mut footer: [String; 16] = Default::default();
    footer[7] = "Average:".into();
    footer[8] = format!("{:?}", Duration::from_nanos(averages[0] as u64));
    footer[9] = format!("{:?}", Duration::from_nanos(averages[1] as u64));
    footer[10] = format!("{:?}", Duration::from_nanos(averages[2] as u64));
    averages
        .iter()
        .skip(3)
        .enumerate()
        .for_each(|(i, a)| footer[i + 11] = format!("{a:.2}"));

    metrics_table.footer = Some(footer);

    Ok(metrics_table)
}
