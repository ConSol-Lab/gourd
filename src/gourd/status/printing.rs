use std::cmp::max;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::Write;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use gourd_lib::constants::ERROR_STYLE;
use gourd_lib::constants::NAME_STYLE;
use gourd_lib::constants::PRIMARY_STYLE;
use gourd_lib::constants::SHORTEN_STATUS_CUTOFF;
use gourd_lib::constants::TERTIARY_STYLE;
use gourd_lib::constants::WARNING_STYLE;
use gourd_lib::ctx;
use gourd_lib::experiment::Environment;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::FieldRef;
use gourd_lib::experiment::Run;
use log::info;

use super::ExperimentStatus;
use super::FsState;
use super::SlurmState;
use super::Status;

#[cfg(not(tarpaulin_include))] // There are no meaningful tests for an enum's Display implementation

impl Display for SlurmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlurmState::BootFail => write!(f, "{ERROR_STYLE}boot failed{ERROR_STYLE:#}"),
            SlurmState::Cancelled => {
                write!(f, "{WARNING_STYLE}slurm job cancelled{WARNING_STYLE:#}")
            }
            SlurmState::Deadline => {
                write!(f, "{ERROR_STYLE}slurm job deadline reached{ERROR_STYLE:#}")
            }
            SlurmState::NodeFail => write!(f, "{ERROR_STYLE}slurm node failed{ERROR_STYLE:#}"),
            SlurmState::OutOfMemory => {
                write!(f, "{WARNING_STYLE}slurm job out of memory{WARNING_STYLE:#}")
            }
            SlurmState::Preempted => write!(f, "{ERROR_STYLE}slurm job preempted{ERROR_STYLE:#}"),
            SlurmState::Suspended => write!(f, "{ERROR_STYLE}slurm job suspended{ERROR_STYLE:#}"),
            SlurmState::Timeout => write!(f, "{WARNING_STYLE}slurm job timed out{WARNING_STYLE:#}"),
            SlurmState::SlurmFail => write!(f, "{ERROR_STYLE}slurm job failed{ERROR_STYLE:#}"),
            SlurmState::Success => write!(f, "{PRIMARY_STYLE}job finished!{PRIMARY_STYLE:#}"),
            SlurmState::Pending => write!(f, "{TERTIARY_STYLE}pending..{TERTIARY_STYLE:#}"),
            SlurmState::Running => write!(f, "{TERTIARY_STYLE}running...{TERTIARY_STYLE:#}"),
        }
    }
}

impl Display for FsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsState::Pending => write!(f, "pending?"),
            FsState::Running => write!(f, "running!"),
            FsState::Completed(metrics) => {
                if metrics.exit_code == 0 {
                    if f.alternate() {
                        write!(
                            f,
                            "{}success{:#} {NAME_STYLE}wall clock time{NAME_STYLE:#}: {}",
                            PRIMARY_STYLE,
                            PRIMARY_STYLE,
                            humantime::Duration::from(metrics.wall_micros)
                        )
                    } else {
                        write!(
                            f,
                            "{}success{:#}, took: {}",
                            PRIMARY_STYLE,
                            PRIMARY_STYLE,
                            humantime::Duration::from(metrics.wall_micros)
                        )
                    }
                } else {
                    write!(
                        f,
                        "{}failed, code: {}{:#}",
                        ERROR_STYLE, metrics.exit_code, ERROR_STYLE
                    )
                }
            }
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            // Long status.
            writeln!(
                f,
                "{NAME_STYLE}file status?{NAME_STYLE:#} {:#}",
                self.fs_status.completion
            )?;

            if let Some(slurm) = &self.slurm_status {
                if slurm.completion.is_completed() {
                    writeln!(
                        f,
                        "{NAME_STYLE}slurm status?{NAME_STYLE:#} {:#} with exit code {}",
                        slurm.completion, slurm.exit_code_slurm
                    )?;
                } else {
                    writeln!(
                        f,
                        "{NAME_STYLE}slurm status?{NAME_STYLE:#} {:#}",
                        slurm.completion
                    )?;
                }
            }

            if let FsState::Completed(measurement) = self.fs_status.completion {
                if let Some(rusage) = measurement.rusage {
                    write!(f, "{NAME_STYLE}metrics{NAME_STYLE:#}:\n{rusage}")?;
                }
            }
        } else {
            // Short summary.
            write!(f, "{}", self.fs_status.completion)?;
        }

        Ok(())
    }
}

/// Display the status of an experiment in a human readable from.
///
/// Returns how many jobs are finished.
#[cfg(not(tarpaulin_include))] // We won't test stdout
pub fn display_statuses(
    f: &mut impl Write,
    experiment: &Experiment,
    statuses: &ExperimentStatus,
    full: bool,
) -> Result<usize> {
    if experiment.runs.len() <= SHORTEN_STATUS_CUTOFF || full {
        long_status(f, experiment, statuses)?;
    } else {
        short_status(f, experiment, statuses)?;
    }

    let mut finished = 0;

    for run in 0..experiment.runs.len() {
        if statuses[&run].is_completed() {
            finished += 1;
        }
    }

    Ok(finished)
}

/// Display a shortened status for a lot of runs.
#[cfg(not(tarpaulin_include))] // We can't test stdout
fn short_status(
    f: &mut impl Write,
    experiment: &Experiment,
    statuses: &ExperimentStatus,
) -> Result<()> {
    let runs = &experiment.runs;

    writeln!(f, "There are {} runs in total", runs.len())?;
    writeln!(f, "Showing shortened output...")?;

    let mut by_program: BTreeMap<String, (usize, usize, usize, usize)> = BTreeMap::new();

    for (run_id, run_data) in runs.iter().enumerate() {
        if !by_program.contains_key(&run_data.program.to_string()) {
            by_program.insert(run_data.program.clone().to_string(), (0, 0, 0, 0));
        }

        if let Some(for_this_prog) = by_program.get_mut(&run_data.program.to_string()) {
            let status = statuses[&run_id].clone();

            if status.is_completed() {
                for_this_prog.0 += 1;
            }

            if status.has_failed(experiment) {
                for_this_prog.1 += 1;
            }

            if status.is_scheduled() {
                for_this_prog.2 += 1;
            }

            for_this_prog.3 += 1;
        }
    }

    for (prog, (completed, failed, sched, total)) in by_program {
        writeln!(f)?;

        writeln!(f, "For program {}:", prog)?;

        if experiment.env == Environment::Slurm {
            writeln!(f, "  {} jobs have been scheduled", sched)?;
        } else {
            writeln!(f, "  {} runs have been created", total)?;
        }
        writeln!(
            f,
            "  ... {} of which have {TERTIARY_STYLE}completed{TERTIARY_STYLE:#}",
            completed
        )?;
        writeln!(
            f,
            "  ... {} of which have {ERROR_STYLE}failed{ERROR_STYLE:#}",
            failed
        )?;
        writeln!(
            f,
            "  ... {} of which have {PRIMARY_STYLE}succeded{PRIMARY_STYLE:#}",
            completed - failed
        )?;
        if experiment.env == Environment::Slurm {
            writeln!(f, "  {} jobs need to still be scheduled", total - sched)?;
        }
    }

    Ok(())
}

/// For an input, decide how its shown to a user.
fn format_input_name(run: &Run) -> String {
    if let Some(input_name) = &run.generated_from_input {
        input_name.clone()
    } else if let Some(parent_id) = run.parent {
        format!("postprocessing of {}", parent_id)
    } else {
        unreachable!("A run cannot spawn out of thin air!");
    }
}

/// Display a shortened status for a small amount of runs.
#[cfg(not(tarpaulin_include))] // We can't test stdout
fn long_status(
    f: &mut impl Write,
    experiment: &Experiment,
    statuses: &ExperimentStatus,
) -> Result<()> {
    let runs = &experiment.runs;

    let mut by_program: BTreeMap<FieldRef, Vec<usize>> = BTreeMap::new();

    let mut longest_input: usize = 0;
    let mut longest_index: usize = 0;

    for (run_id, run_data) in runs.iter().enumerate() {
        longest_input = max(longest_input, format_input_name(run_data).chars().count());
        longest_index = max(longest_index, run_id.to_string().len());

        let prog_name = experiment.get_program(run_data)?.name;

        if let Some(for_this_prog) = by_program.get_mut(&prog_name) {
            for_this_prog.push(run_id);
        } else {
            by_program.insert(prog_name, vec![run_id]);
        }
    }

    for (prog, prog_runs) in by_program {
        writeln!(f)?;

        writeln!(f, "For program {}:", prog)?;

        for run_id in prog_runs {
            let run = &experiment.runs[run_id];
            let status = statuses[&run_id].clone();

            write!(
                f,
                "  {: >numw$}. {NAME_STYLE}{:.<width$}{NAME_STYLE:#}.... {}",
                run_id,
                format_input_name(run),
                if let Some(r) = run.rerun {
                    format!("reran as {NAME_STYLE}{r}{NAME_STYLE:#}")
                } else {
                    format!("{}", status)
                },
                width = longest_input,
                numw = longest_index
            )?;

            if status.fs_status.completion == FsState::Pending {
                if let Some(ss) = &status.slurm_status {
                    write!(f, " on slurm: {}", ss.completion)?;
                } else if run.slurm_id.is_some() && experiment.env == Environment::Slurm {
                    write!(f, " {WARNING_STYLE}not found on slurm{WARNING_STYLE:#}")?;
                } else if run.slurm_id.is_some() && experiment.env == Environment::Local {
                    write!(f, " {WARNING_STYLE}queued!{WARNING_STYLE:#}")?;
                }
            }

            writeln!(f)?;

            if let Some(Some(label_text)) = &status.fs_status.afterscript_completion {
                let display_style = if experiment.labels.map[label_text].rerun_by_default {
                    ERROR_STYLE
                } else {
                    PRIMARY_STYLE
                };

                write!(
                    f,
                    "  {: >numw$}a {:.<width$}.... \
                            label: {display_style}{label_text}{display_style:#}",
                    run_id,
                    "afterscript",
                    numw = longest_index,
                    width = longest_input,
                )?;

                writeln!(f)?;
            } else if let Some(None) = &status.fs_status.afterscript_completion {
                write!(
                    f,
                    "  {: >numw$}a {TERTIARY_STYLE}afterscript ran \
                            successfully{TERTIARY_STYLE:#}",
                    run_id,
                    numw = longest_index,
                )?;

                writeln!(f)?;
            }
        }
    }

    writeln!(f)?;

    Ok(())
}

/// Display the status of an experiment in a human readable from.
#[cfg(not(tarpaulin_include))] // We can't test stdout
pub fn display_job(
    f: &mut impl Write,
    exp: &Experiment,
    statuses: &ExperimentStatus,
    id: usize,
) -> Result<()> {
    info!(
        "Displaying the status of job {} in experiment {}",
        id, exp.seq
    );

    writeln!(f)?;

    if let Some(run) = exp.runs.get(id) {
        let program = exp.get_program(run)?;

        writeln!(f, "{NAME_STYLE}program{NAME_STYLE:#}: {}", run.program)?;
        writeln!(
            f,
            "  {NAME_STYLE}binary{NAME_STYLE:#}: {:?}",
            program.binary
        )?;

        writeln!(f, "{NAME_STYLE}input{NAME_STYLE:#}: {:?}", run.input.file)?;

        if let Some(input_file) = &run.input.file {
            writeln!(f, "  {NAME_STYLE}file{NAME_STYLE:#}:   {:?}", input_file)?;
        }

        writeln!(f, "{NAME_STYLE}arguments{NAME_STYLE:#}: {:?}\n", run.input)?;

        writeln!(
            f,
            "{NAME_STYLE}output path{NAME_STYLE:#}: {:?}",
            run.output_path
        )?;
        writeln!(
            f,
            "{NAME_STYLE}stderr path{NAME_STYLE:#}: {:?}",
            run.err_path
        )?;
        writeln!(
            f,
            "{NAME_STYLE}metric path{NAME_STYLE:#}: {:?}\n",
            run.metrics_path
        )?;

        if let Some(slurm_id) = &run.slurm_id {
            writeln!(
                f,
                "scheduled on slurm as {}
                  with limits
                 {}
                ",
                slurm_id, run.limits
            )?;
        }

        let status = &statuses[&id];

        writeln!(f, "{status:#}")?;

        if let Some(Some(label_text)) = &status.fs_status.afterscript_completion {
            let display_style = if exp.labels.map[label_text].rerun_by_default {
                ERROR_STYLE
            } else {
                PRIMARY_STYLE
            };

            writeln!(
                f,
                "{NAME_STYLE}afterscript ran and assigned \
                    label{NAME_STYLE:#}: {display_style}{label_text}{display_style:#}",
            )?;

            writeln!(f)?;
        } else if let Some(None) = &status.fs_status.afterscript_completion {
            writeln!(
                f,
                "{TERTIARY_STYLE}afterscript ran successfully{TERTIARY_STYLE:#}",
            )?;

            writeln!(f)?;
        }

        if let Some(new_id) = run.rerun {
            writeln!(
                f,
                "{NAME_STYLE}this job has been reran as {new_id}{NAME_STYLE:#}",
            )?;

            writeln!(f)?;
        }

        Ok(())
    } else {
        Err(anyhow!("A run with this id does not exist")).with_context(ctx!(
            "", ;
            "You can see the run ids by running {}gourd status{:#}", PRIMARY_STYLE, PRIMARY_STYLE
        ))
    }
}
