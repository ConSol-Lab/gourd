use std::cmp::max;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::Write;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use gourd_lib::constants::CMD_DOC_STYLE;
use gourd_lib::constants::ERROR_STYLE;
use gourd_lib::constants::NAME_STYLE;
use gourd_lib::constants::PARAGRAPH_STYLE;
use gourd_lib::constants::PATH_STYLE;
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
                if f.sign_minus() {
                    // reduced output, guarantees similar length output to pending? and running!
                    write!(f, "completed")
                } else if metrics.exit_code == 0 {
                    if f.alternate() {
                        write!(
                            f,
                            "{PRIMARY_STYLE}success{PRIMARY_STYLE:#} {NAME_STYLE}wall clock time{NAME_STYLE:#}: {}",
                            humantime::Duration::from(metrics.wall_micros)
                        )
                    } else {
                        write!(
                            f,
                            "{PRIMARY_STYLE}success{PRIMARY_STYLE:#}, took: {}",
                            humantime::Duration::from(metrics.wall_micros)
                        )
                    }
                } else {
                    write!(
                        f,
                        "{ERROR_STYLE}failed, code: {}{ERROR_STYLE:#}",
                        metrics.exit_code,
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
pub fn display_statuses(
    f: &mut impl Write,
    experiment: &Experiment,
    statuses: &ExperimentStatus,
    full: bool,
) -> Result<usize> {
    if full || experiment.runs.len() <= SHORTEN_STATUS_CUTOFF {
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
        let prog = experiment.programs[run_data.program].name.clone();
        match by_program.entry(prog) {
            Entry::Vacant(e) => {
                e.insert((0, 0, 0, 0));
            }
            Entry::Occupied(mut o) => {
                let mut for_this_prog = *o.get();

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

                o.insert(for_this_prog);
            }
        }
    }

    for (prog, (completed, failed, sched, total)) in by_program {
        writeln!(f)?;

        writeln!(f, "For program {prog}:")?;

        if experiment.env == Environment::Slurm {
            writeln!(f, "  {sched} jobs have been scheduled",)?;
        } else {
            writeln!(f, "  {total} runs have been created",)?;
        }
        writeln!(
            f,
            "  ... {completed} of which have {TERTIARY_STYLE}completed{TERTIARY_STYLE:#}",
        )?;
        writeln!(
            f,
            "  ... {failed} of which have {ERROR_STYLE}failed{ERROR_STYLE:#}",
        )?;
        writeln!(
            f,
            "  ... {} of which have {PRIMARY_STYLE}succeded{PRIMARY_STYLE:#}",
            completed - failed
        )?;
        if experiment.env == Environment::Slurm {
            writeln!(f, "  {} jobs still need to be scheduled", total - sched)?;
        }
    }

    Ok(())
}

/// For an input, decide how it's shown to a user.
fn format_input_name(exp: &Experiment, run: &Run, grouped: bool) -> String {
    if !grouped {
        if let Some(input_name) = &run.generated_from_input {
            input_name.clone()
        } else if let Some(parent_id) = run.parent {
            format!("postprocessing of {parent_id}",)
        } else {
            unreachable!("A run cannot spawn out of thin air!");
        }
    } else if let Some(input_name) = &run.generated_from_input {
        format!("{} ({input_name})", exp.programs[run.program].name,)
    } else {
        // when this function was implemented this branch was unreachable,
        // but it is reasonable that this will change in the future, and not
        // panicking here seems reasonable.
        format_input_name(exp, run, false)
    }
}

/// Display a shortened status for a small amount of runs.
fn long_status(
    f: &mut impl Write,
    experiment: &Experiment,
    statuses: &ExperimentStatus,
) -> Result<()> {
    let runs = &experiment.runs;

    // map from programs to run ids
    let mut by_program: BTreeMap<FieldRef, Vec<usize>> = BTreeMap::new();

    let mut longest_input: usize = 0;
    let mut longest_index: usize = 0;

    let mut grouped_runs: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for (run_id, run_data) in runs.iter().enumerate() {
        longest_input = max(
            longest_input,
            format_input_name(experiment, run_data, run_data.group.is_some())
                .chars()
                .count(),
        );
        longest_index = max(longest_index, run_id.to_string().len());

        if let Some(group) = &run_data.group {
            if let Some(for_this_group) = grouped_runs.get_mut(group) {
                for_this_group.push(run_id);
            } else {
                grouped_runs.insert(group.clone(), vec![run_id]);
            }
        } else {
            let prog_name = experiment.get_program(run_data)?.name;

            if let Some(for_this_prog) = by_program.get_mut(&prog_name) {
                for_this_prog.push(run_id);
            } else {
                by_program.insert(prog_name, vec![run_id]);
            }
        }
    }

    for (prog, prog_runs) in by_program {
        writeln!(f)?;

        writeln!(f, "For program {prog}:")?;

        display_runs(
            false,
            f,
            experiment,
            statuses,
            prog_runs,
            longest_input,
            longest_index,
        )?;
    }

    for (prog, prog_runs) in grouped_runs {
        writeln!(f)?;

        writeln!(f, "For group {prog}:")?;

        display_runs(
            true,
            f,
            experiment,
            statuses,
            prog_runs,
            longest_input,
            longest_index,
        )?;
    }

    writeln!(f)?;

    Ok(())
}

/// Display runs of some group.
fn display_runs(
    group: bool,
    f: &mut impl Write,
    experiment: &Experiment,
    statuses: &ExperimentStatus,
    prog_runs: Vec<usize>,
    longest_input: usize,
    longest_index: usize,
) -> Result<()> {
    for run_id in prog_runs {
        let run = &experiment.runs[run_id];
        let status = statuses[&run_id].clone();

        write!(
            f,
            "  {run_id: >numw$}. {NAME_STYLE}{:.<width$}{NAME_STYLE:#}.... {}",
            format_input_name(experiment, run, group),
            if let Some(r) = run.rerun {
                format!("reran as {NAME_STYLE}{r}{NAME_STYLE:#}")
            } else {
                format!("{status}")
            },
            width = longest_input,
            numw = longest_index
        )?;

        if status.fs_status.completion == FsState::Pending {
            if let Some(ss) = &status.slurm_status {
                write!(f, " on slurm: {}", ss.completion)?;
            } else if let Some(slurm_id) = &run.slurm_id {
                write!(
                    f,
                    " on slurm with job id {WARNING_STYLE}{slurm_id}{WARNING_STYLE:#}"
                )?;
            } else if run.slurm_id.is_some() && experiment.env == Environment::Local {
                write!(f, " {WARNING_STYLE}queued!{WARNING_STYLE:#}")?;
            }
        }

        writeln!(f)?;

        if let Some(Some(label_text)) = &status.fs_status.afterscript_completion {
            let display_style = if experiment.labels[label_text].rerun_by_default {
                ERROR_STYLE
            } else {
                PRIMARY_STYLE
            };

            write!(
                f,
                "  {run_id: >numw$}a {:.<width$}.... \
                            label: {display_style}{label_text}{display_style:#}",
                "afterscript",
                numw = longest_index,
                width = longest_input,
            )?;

            writeln!(f)?;
        } else if let Some(None) = &status.fs_status.afterscript_completion {
            write!(
                f,
                "  {run_id: >longest_index$}a {TERTIARY_STYLE}afterscript ran \
                            successfully{TERTIARY_STYLE:#}"
            )?;

            writeln!(f)?;
        }
    }

    Ok(())
}

/// Display the status of an experiment in a human-readable from.
pub fn display_job(
    f: &mut impl Write,
    exp: &Experiment,
    statuses: &ExperimentStatus,
    id: usize,
) -> Result<()> {
    use gourd_lib::constants::TRUNCATE_AFTERSCRIPT_OUTPUT;
    use log::debug;

    info!(
        "Displaying the status of job {id} in experiment {}",
        exp.seq
    );

    writeln!(f)?;

    if let Some(run) = exp.runs.get(id) {
        let program = exp.get_program(run)?;

        writeln!(f, "{NAME_STYLE}program{NAME_STYLE:#}: {}", program.name)?;
        writeln!(
            f,
            "  {NAME_STYLE}binary{NAME_STYLE:#}: {PATH_STYLE}{}{PATH_STYLE:#}",
            program.binary.display()
        )?;

        writeln!(f, "{NAME_STYLE}input{NAME_STYLE:#}:")?;

        writeln!(
            f,
            "  {NAME_STYLE}file{NAME_STYLE:#}: {PATH_STYLE}{}{PATH_STYLE:#}",
            run.input
                .file
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or("None".to_string())
        )?;

        writeln!(
            f,
            "  {NAME_STYLE}arguments{NAME_STYLE:#}: {:?}\n",
            run.input.args
        )?;

        if let Some(group) = &run.group {
            writeln!(f, "{NAME_STYLE}group{NAME_STYLE:#}: {group}")?;
        }

        writeln!(
            f,
            "{NAME_STYLE}output path{NAME_STYLE:#}: {PATH_STYLE}{}{PATH_STYLE:#}",
            run.output_path.display()
        )?;
        writeln!(
            f,
            "{NAME_STYLE}stderr path{NAME_STYLE:#}: {PATH_STYLE}{}{PATH_STYLE:#}",
            run.err_path.display()
        )?;
        writeln!(
            f,
            "{NAME_STYLE}metric path{NAME_STYLE:#}: {PATH_STYLE}{}{PATH_STYLE:#}\n",
            run.metrics_path.display()
        )?;

        if let Some(slurm_id) = &run.slurm_id {
            writeln!(
                f,
                "scheduled on slurm as {TERTIARY_STYLE}{slurm_id}{TERTIARY_STYLE:#}\nwith limits\n{}",
                 run.limits
            )?;

            if let Some(slurm_file) = &statuses[&id].slurm_file_text {
                let slurm_out = exp
                    .slurm_out(slurm_id)
                    .ok_or(anyhow!("Slurm config not found (unreachable)"))?;
                let slurm_err = exp
                    .slurm_err(slurm_id)
                    .ok_or(anyhow!("Slurm config not found (unreachable)"))?;
                writeln!(
                    f,
                    "{NAME_STYLE}Slurm job stdout{NAME_STYLE:#} ({PATH_STYLE}{}{PATH_STYLE:#}):
\"{PARAGRAPH_STYLE}{}{PARAGRAPH_STYLE:#}\"",
                    slurm_out.display(),
                    slurm_file.stdout.trim()
                )?;
                writeln!(
                    f,
                    "{NAME_STYLE}Slurm job stderr{NAME_STYLE:#} ({PATH_STYLE}{}{PATH_STYLE:#}):
\"{PARAGRAPH_STYLE}{}{PARAGRAPH_STYLE:#}\"",
                    slurm_err.display(),
                    slurm_file.stderr.trim()
                )?;
            }
        }

        let status = &statuses[&id];

        writeln!(f, "{status:#}")?;

        if let Some(Some(label_text)) = &status.fs_status.afterscript_completion {
            let display_style = if exp.labels[label_text].rerun_by_default {
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
            if let Some(mut out) = exp.runs[id].afterscript_output.clone() {
                writeln!(
                    f,
                    "{TERTIARY_STYLE}afterscript ran successfully{TERTIARY_STYLE:#}",
                )?;

                let truncate_output = |x: &mut String| {
                    let mut touch = false;
                    if x.len() > TRUNCATE_AFTERSCRIPT_OUTPUT.0 {
                        touch = true;
                        x.truncate(TRUNCATE_AFTERSCRIPT_OUTPUT.0);
                    }
                    if x.lines().count() > TRUNCATE_AFTERSCRIPT_OUTPUT.1 {
                        *x = x.lines().take(TRUNCATE_AFTERSCRIPT_OUTPUT.1).collect();
                        touch = true;
                    }
                    touch
                };

                if truncate_output(&mut out) {
                    debug!("truncating afterscript output for gourd status -i {id}");
                    writeln!(
                        f,
                        "afterscript output was too long, run {CMD_DOC_STYLE} gourd status {} -i {id} --after-out {CMD_DOC_STYLE:#} to view entire output

shortened output:\n{PARAGRAPH_STYLE}{out}[truncated]{PARAGRAPH_STYLE:#}",
                        exp.seq,
                    )?;
                } else {
                    writeln!(
                        f,
                        "afterscript output:\n{PARAGRAPH_STYLE}{out}{PARAGRAPH_STYLE:#}",
                    )?;
                }

                writeln!(f)?;
            } else {
                unreachable!(
                    "this is not supposed to happen, \
                    please contact the developers on \
                    https://github.com/ConSol-Lab/gourd/issues/new \
                    with screenshots (AFT_COMPL_OUT_IS_NONE)"
                );
            }
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
