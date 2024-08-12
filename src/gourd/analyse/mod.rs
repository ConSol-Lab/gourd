use std::cmp::max;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use csv::Writer;
use gourd_lib::bailc;
use gourd_lib::constants::PLOT_SIZE;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::FieldRef;
use gourd_lib::measurement::RUsage;
use log::debug;
use plotters::prelude::*;
use plotters::style::register_font;
use plotters::style::BLACK;

use crate::status::FileSystemBasedStatus;
use crate::status::FsState;
use crate::status::SlurmBasedStatus;
use crate::status::Status;

/// Plot width, size, and data to plot.
type PlotData = (u128, u128, BTreeMap<FieldRef, Vec<(u128, u128)>>);

/// Collect and export metrics.
pub fn analysis_csv(path: &Path, statuses: BTreeMap<usize, Status>) -> Result<()> {
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

    writer.write_record(header)?;

    for (id, status) in statuses {
        let fs_status = &status.fs_status;
        let slurm_status = status.slurm_status;

        let mut record = get_fs_status_info(id, fs_status);
        record.append(&mut get_afterscript_output_info(
            &status.fs_status.afterscript_completion,
        ));
        record.append(&mut get_slurm_status_info(&slurm_status));

        writer.write_record(record)?;
    }

    writer.flush()?;

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
            "running".to_string(),
            "...".to_string(),
            "...".to_string(),
            "...".to_string(),
        ],
        FsState::Completed(measurement) => {
            vec![
                "completed".to_string(),
                format!("{:?}", measurement.wall_micros),
                format!("{:?}", measurement.exit_code),
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
        } else {
            vec![String::from("done, no label")]
        }
    } else {
        vec![String::from("no afterscript")]
    }
}

/// Get data for plotting and generate plots.
pub fn analysis_plot(
    path: &Path,
    statuses: BTreeMap<usize, Status>,
    experiment: Experiment,
    is_png: bool,
) -> Result<()> {
    let completions = get_completions(statuses, experiment)?;

    let data = get_data_for_plot(completions);

    if is_png {
        make_plot(data, BitMapBackend::new(&path, PLOT_SIZE))?;
    } else {
        make_plot(data, SVGBackend::new(&path, PLOT_SIZE))?;
    }

    Ok(())
}

/// Get completion times of jobs.
pub fn get_completions(
    statuses: BTreeMap<usize, Status>,
    experiment: Experiment,
) -> Result<BTreeMap<FieldRef, Vec<u128>>> {
    let mut completions: BTreeMap<FieldRef, Vec<u128>> = BTreeMap::new();

    for (id, status) in statuses {
        let program_name = experiment.program_from_run_id(id)?.name;

        if status.is_completed() {
            let time = match get_completion_time(status.fs_status.completion) {
                Ok(t) => t.as_nanos(),
                // No RUsage
                Err(_) => continue,
            };

            if completions.contains_key(&program_name) {
                let mut times = completions[&program_name].clone();
                times.push(time);
                completions.insert(program_name.clone(), times);
            } else {
                completions.insert(program_name.clone(), vec![time]);
            }
        }
    }

    for times in completions.values_mut() {
        times.sort();
    }
    Ok(completions)
}

/// Get completion time of a run.
pub fn get_completion_time(state: FsState) -> Result<Duration> {
    match state {
        FsState::Completed(measured) => {
            let measured = measured.rusage;

            if let Some(r) = measured {
                Ok(r.utime)
            } else {
                bailc!("RUsage is not accessible even though the run completed");
            }
        }
        _ => {
            bailc!("Run was supposed to be completed");
        }
    }
}

/// Get wall clock data for cactus plot.
pub fn get_data_for_plot(completions: BTreeMap<FieldRef, Vec<u128>>) -> PlotData {
    let max_time = completions.values().flatten().max();
    let mut data = BTreeMap::new();

    if max_time.is_some() {
        let max_time = *max_time.unwrap();
        let mut max_count = 0;

        for (name, program) in completions {
            let mut data_per_program = vec![];
            let mut already_finished = 0;

            for end in program {
                if end > 0 {
                    data_per_program.push((end - 1, already_finished));
                }

                already_finished += 1;
                data_per_program.push((end, already_finished));
            }

            data_per_program.push((max_time, already_finished));

            max_count = max(max_count, already_finished);

            data.insert(name, data_per_program);
        }

        (max_time, max_count, data)
    } else {
        (0, 0, data)
    }
}

/// Plot the results of runs in a cactus plot.
pub fn make_plot<T>(plot_data: PlotData, backend: T) -> Result<()>
where
    T: DrawingBackend,
    <T as DrawingBackend>::ErrorType: 'static,
{
    debug!("Drawing a plot");

    let (max_time, max_count, cactus_data) = plot_data;

    register_font(
        "sans-serif",
        FontStyle::Normal,
        include_bytes!("../../resources/LinLibertine_R.otf"),
    )
    .map_err(|_| anyhow!("Could not load the font"))?;

    let style = TextStyle::from(("sans-serif", 20).into_font()).color(&BLACK);
    let root = backend.into_drawing_area();

    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption("Cactus plot", 40)
        .build_cartesian_2d(0..max_time + 1, 0..max_count + 1)?;

    chart
        .configure_mesh()
        .light_line_style(WHITE)
        .x_label_style(style.clone())
        .y_label_style(style.clone())
        .label_style(style.clone())
        .x_desc("Nanoseconds")
        .y_desc("Runs")
        .draw()?;

    for (idx, (name, datas)) in (0..).zip(cactus_data) {
        chart
            .draw_series(LineSeries::new(
                datas,
                Into::<ShapeStyle>::into(Palette99::pick(idx)).stroke_width(3),
            ))?
            .label(format!("{}", name))
            .legend(move |(x, y)| {
                Rectangle::new(
                    [(x - 5, y - 5), (x + 5, y + 5)],
                    Palette99::pick(idx).stroke_width(5),
                )
            });
    }

    chart.configure_series_labels().label_font(style).draw()?;

    root.present()?;

    Ok(())
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
