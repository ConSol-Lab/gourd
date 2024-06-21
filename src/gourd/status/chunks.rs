use anyhow::Result;
use gourd_lib::constants::PRIMARY_STYLE;
use gourd_lib::constants::SCHEDULE_BAR_WIDTH;
use gourd_lib::constants::TERTIARY_STYLE;
use gourd_lib::experiment::Experiment;
use log::info;

use crate::slurm::chunk::Chunkable;

/// Print user readable infomation about the scheduling status.
pub fn print_scheduling(exp: &Experiment, starting: bool) -> Result<()> {
    if starting {
        info!(
            "Starting out experiment {PRIMARY_STYLE}{}{PRIMARY_STYLE:#}...",
            exp.seq
        );
    } else {
        info!(
            "Continuing experiment {PRIMARY_STYLE}{}{PRIMARY_STYLE:#}...",
            exp.seq
        );
    }

    let total_runs: usize = exp.runs.len();

    let total_scheduled: usize = total_runs - exp.get_unscheduled_runs()?.len();

    let total_chunks: usize = exp.chunks.len();

    info!("There are {total_runs} total runs which are spread over {total_chunks} chunks");
    info!(
        "Out of these {total_scheduled} have been scheduled and {} are still left unscheduled",
        total_runs - total_scheduled
    );

    if total_scheduled == total_runs {
        info!("Nothing more to schedule");
        return Ok(());
    }

    info!("");

    let mut bar = String::default();
    let percentage: f64 = (total_scheduled as f64) / (total_runs as f64);
    let filled = f64::ceil(percentage * (SCHEDULE_BAR_WIDTH as f64)) as usize;
    let unfilled = SCHEDULE_BAR_WIDTH - filled;

    bar.push_str(&format!("  [{PRIMARY_STYLE}"));

    for _ in 0..filled {
        bar.push('#');
    }

    bar.push_str(&format!("{PRIMARY_STYLE:#}{TERTIARY_STYLE}"));

    for _ in 0..unfilled {
        bar.push('.');
    }

    bar.push_str(&format!("{TERTIARY_STYLE:#}]"));

    let mut bar_lower = String::default();

    bar_lower.push_str(&format!("   {PRIMARY_STYLE}"));

    for _ in 0..filled {
        bar_lower.push('^');
    }

    bar_lower.push_str(&format!(
        " these ones are already sent to slurm{PRIMARY_STYLE:#}"
    ));

    info!("{}", bar);

    if total_scheduled > 0 {
        info!("{}", bar_lower);
    }

    info!("");
    info!("To schedule the rest when this part finishes run {PRIMARY_STYLE}gourd continue{PRIMARY_STYLE:#}");

    Ok(())
}

#[cfg(test)]
#[path = "tests/chunks.rs"]
mod tests;
