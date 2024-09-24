use std::cmp::max;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use gourd_lib::constants::PLOT_SIZE;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::FieldRef;
use log::debug;
use plotters::backend::BitMapBackend;
use plotters::backend::DrawingBackend;
use plotters::backend::SVGBackend;
use plotters::chart::ChartBuilder;
use plotters::drawing::IntoDrawingArea;
use plotters::element::Rectangle;
use plotters::prelude::*;
use plotters::style::register_font;
use plotters::style::Palette;

use crate::analyse::get_completions;
use crate::cli::def::PlotType;
use crate::status::ExperimentStatus;

/// Plot width, size, and data to plot.
pub(super) type PlotData = (u128, u128, BTreeMap<FieldRef, Vec<(u128, u128)>>);

/// Get data for plotting and generate plots.
pub fn analysis_plot(
    path: &Path,
    statuses: ExperimentStatus,
    experiment: &Experiment,
    plot_type: PlotType,
) -> Result<PathBuf> {
    let completions = get_completions(statuses, experiment)?;

    let data = get_data_for_plot(completions);

    match plot_type {
        PlotType::Png => make_plot(data, BitMapBackend::new(&path, PLOT_SIZE))?,
        PlotType::Svg => make_plot(data, SVGBackend::new(&path, PLOT_SIZE))?,
        // PlotType::Csv => bailc!("Plotting in CSV is not yet implemented!"),
    }

    Ok(path.into())
}

/// Get wall clock data for cactus plot.
pub fn get_data_for_plot(completions: BTreeMap<FieldRef, Vec<u128>>) -> PlotData {
    let max_time = completions.values().flatten().max();
    let mut data = BTreeMap::new();

    if let Some(mt) = max_time {
        let max_time = *mt;
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
            .label(name.to_string())
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
#[path = "tests/plotting.rs"]
mod tests;
