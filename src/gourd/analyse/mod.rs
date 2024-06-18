use std::cmp::max;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use csv::Writer;
use gourd_lib::bailc;
use gourd_lib::ctx;
use gourd_lib::error::Ctx;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::FieldRef;
use gourd_lib::measurement::RUsage;
use plotters::prelude::*;
use plotters::style::register_font;

use crate::status::FileSystemBasedStatus;
use crate::status::FsState;
use crate::status::SlurmBasedStatus;
use crate::status::Status;

/// Plot width, size, and data to plot.
type PlotData = (u128, u128, BTreeMap<String, Vec<(u128, u128)>>);

/// Collect and export metrics.
pub fn analysis_csv(path: &PathBuf, statuses: &BTreeMap<usize, Status>) -> Result<()> {
    let mut writer = Writer::from_path(path)?;

    let header = vec![
        "id".to_string(),
        "file system status".to_string(),
        "wall micros".to_string(),
        "exit code".to_string(),
        "RUsage".to_string(),
        "afterscript output".to_string(),
        "slurm completion".to_string(),
    ];

    writer
        .write_record(header)
        .with_context(ctx!("Could not write to the CSV file at {:?}", path; "",))?;

    for (id, status) in statuses {
        let fs_status = &status.fs_status;
        let slurm_status = status.slurm_status;

        let mut record = get_fs_status_info(*id, fs_status);
        record.append(&mut get_afterscript_output_info(
            &status.fs_status.afterscript_completion,
        ));
        record.append(&mut get_slurm_status_info(&slurm_status));

        writer
            .write_record(record)
            .with_context(ctx!("Could not write to the CSV file at {:?}", path; "",))?;
    }

    writer
        .flush()
        .with_context(ctx!("Could not write to the CSV file at {:?}", path; "",))?;

    Ok(())
}

/// Gets file system info for CSV.
pub fn get_fs_status_info(id: usize, fs_status: &FileSystemBasedStatus) -> Vec<String> {
    let mut completion = match fs_status.completion {
        FsState::Pending => vec![
            "pending".to_string(),
            "...".to_string(),
            "...".to_string(),
            "...".to_string(),
        ],
        FsState::Running => vec![
            String::from("running"),
            "...".to_string(),
            "...".to_string(),
            "...".to_string(),
        ],
        FsState::Completed(measurement) => {
            vec![
                String::from("completed"),
                format!("{:#?}", measurement.wall_micros),
                format!("{:#?}", measurement.exit_code),
                format_rusage(measurement.rusage),
            ]
        }
    };

    let mut res = vec![id.to_string()];
    res.append(&mut completion);

    res
}

/// Formats RUsage of a run for the CSV.
pub fn format_rusage(rusage: Option<RUsage>) -> String {
    if rusage.is_some() {
        format!("{:#?}", rusage.unwrap())
    } else {
        String::from("none")
    }
}

/// Gets slurm status info for CSV.
pub fn get_slurm_status_info(slurm_status: &Option<SlurmBasedStatus>) -> Vec<String> {
    if let Some(inner) = slurm_status {
        vec![format!("{:#?}", inner.completion)]
    } else {
        vec!["...".to_string()]
    }
}

/// Gets afterscript output info for CSV.
pub fn get_afterscript_output_info(afterscript_completion: &Option<Option<String>>) -> Vec<String> {
    if let Some(inner) = afterscript_completion {
        if let Some(label) = inner {
            vec![label.clone()]
        } 
        else {
            vec![String::from("done, no label")]
        }
    } else {
        vec![String::from("no afterscript")]
    }
}

/// Get data for plotting and generate plots.
pub fn analysis_plot(
    path: &PathBuf,
    statuses: BTreeMap<usize, Status>,
    experiment: Experiment,
) -> Result<()> {
    let completions = get_completions(statuses, experiment)?;

    let (max_time, max_count, data) = get_data_for_plot(completions);

    make_plot(path, data, max_time, max_count)?;

    Ok(())
}

/// Get completion times of jobs.
pub fn get_completions(
    statuses: BTreeMap<usize, Status>,
    experiment: Experiment,
) -> Result<BTreeMap<String, Vec<u128>>> {
    let mut completions: BTreeMap<String, Vec<u128>> = BTreeMap::new();

    for (id, status) in statuses {
        let program_name = match &experiment.runs[id].program {
            FieldRef::Regular(name) => name,
            FieldRef::Postprocess(name) => name,
        };

        if status.is_completed() {
            let time = get_completion_time(status.fs_status.completion)?.as_nanos();

            if completions.contains_key(program_name) {
                let mut times = completions[program_name].clone();
                times.push(time);
                completions.insert(program_name.clone(), times);
            } else {
                completions.insert(program_name.clone(), vec![time]);
            }
        }
    }

    Ok(completions)
}

/// Get completion time of a run.
pub fn get_completion_time(state: FsState) -> Result<Duration> {
    match state {
        FsState::Completed(measured) => {
            let gg = measured.rusage;

            if let Some(r) = gg {
                Ok(r.utime)
            } else {
                bailc!(
                    "RUsage is not accessible even though the run completed", ;
                    "", ;
                    "",
                );
            }
        }
        _ => {
            bailc!(
                "Run was supposed to be completed", ;
                "", ;
                "",
            );
        }
    }
}

/// Get wall clock data for cactus plot.
pub fn get_data_for_plot(completions: BTreeMap<String, Vec<u128>>) -> PlotData {
    let max_time = completions.values().flatten().max();
    let mut data = BTreeMap::new();

    if max_time.is_some() {
        let max_time = *max_time.unwrap();
        let mut max_count = 0;

        for (name, program) in completions {
            let mut data_per_program = vec![];

            for i in 0..=max_time {
                let filtered: Vec<&u128> = program.iter().filter(|x| **x <= i).collect();
                let count = filtered.len() as u128;

                data_per_program.push((i, count));
                max_count = max(max_count, count);
            }
            data.insert(name, data_per_program);
        }

        (max_time, max_count, data)
    } else {
        (0, 0, data)
    }
}

/// Plot the results of runs in a cactus plot.
pub fn make_plot(
    path: &PathBuf,
    cactus_data: BTreeMap<String, Vec<(u128, u128)>>,
    max_time: u128,
    max_count: u128,
) -> Result<()> {
    let _add_font = register_font(
        "sans-serif",
        FontStyle::Normal,
        include_bytes!("Museo_Slab_500.otf"),
    );

    let root = BitMapBackend::new(path, (1920, 1080)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption("Cactus plot", 40)
        .build_cartesian_2d(0..max_time + 1, 0..max_count + 1)?;

    chart.configure_mesh().light_line_style(WHITE).draw()?;

    for (idx, (_name, datas)) in (0..).zip(cactus_data) {
        chart.draw_series(LineSeries::new(datas, &Palette99::pick(idx)))?;
        // .label(format!("program {}", idx))
        // .legend(move |(x, y)| {
        //     Rectangle::new([(x - 5, y - 5), (x + 5, y + 5)],
        // Palette99::pick(idx)) });
    }

    root.present()?;

    Ok(())
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
