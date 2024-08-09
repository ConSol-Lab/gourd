use std::collections::BTreeMap;

use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::constants::CMD_STYLE;
use gourd_lib::constants::HELP_STYLE;
use gourd_lib::constants::NAME_STYLE;
use gourd_lib::constants::RERUN_LIST_PROMPT_CUTOFF;
use gourd_lib::experiment::Environment;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use inquire::Select;
use log::debug;
use log::trace;

use crate::cli::printing::query_update_resource_limits;
use crate::cli::printing::query_yes_no;
use crate::init::interactive::ask;
use crate::rerun::runs::re_runnable;
use crate::rerun::status::status_of_single_run;
use crate::rerun::update_program_resource_limits;
use crate::rerun::RerunStatus;
use crate::status::DynamicStatus;
use crate::status::ExperimentStatus;
use crate::status::SlurmState;

/// Ask the user if they want to change any resource limits for the current
/// experiment.
pub fn query_changing_resource_limits(
    experiment: &mut Experiment,
    script_mode: bool,
    selected_runs: &[usize],
    file_system: &mut impl FileOperations,
) -> Result<()> {
    if experiment.env == Environment::Slurm && !script_mode {
        let statuses = experiment.status(file_system)?;
        let (out_of_memory, out_of_time) =
            selected_runs
                .iter()
                .map(|r| statuses[r].clone())
                .fold((0, 0), |(oom, oot), s| match s.slurm_status {
                    Some(s) => match s.completion {
                        SlurmState::OutOfMemory => (oom + 1, oot),
                        SlurmState::Timeout => (oom, oot + 1),
                        _ => (oom, oot),
                    },
                    None => (oom, oot),
                });

        if query_yes_no(&format!(
            "{} runs ran out of memory and {} runs ran out of time. \
                     Do you want to change the resource limits for their programs?",
            out_of_memory, out_of_time
        ))? {
            query_changing_limits_for_programs(selected_runs, experiment)?;
        }
    }
    Ok(())
}

/// Check the status of a single run and ask the user what to rerun.
pub(super) fn check_single_run_failed(
    specific_run: &usize,
    experiment: &Experiment,
    statuses: &ExperimentStatus,
) -> Result<usize> {
    match status_of_single_run(specific_run, statuses, experiment)? {
        RerunStatus::NotFinished => {
            if !query_yes_no(&format!(
                "Run {specific_run} has not completed yet, \
                are you sure? \
                You can cancel this run with \
                {CMD_STYLE}gourd cancel -i {specific_run}{CMD_STYLE:#}"
            ))? {
                bailc!("Rerun cancelled..", ; "", ; "",);
            } else {
                Ok(*specific_run)
            }
        }

        RerunStatus::FinishedExitZero => {
            if !query_yes_no(&format!(
                "Run {specific_run} has finished successfully, are you sure?"
            ))? {
                bailc!("Rerun cancelled.")
            } else {
                Ok(*specific_run)
            }
        }

        RerunStatus::FinishedSuccessLabel(l) => {
            if !query_yes_no(&format!(
                "Run {specific_run} has finished successfully, \
                with label {NAME_STYLE}{l}{NAME_STYLE:#}. Are you sure?"
            ))? {
                bailc!("Rerun cancelled.")
            } else {
                Ok(*specific_run)
            }
        }

        RerunStatus::FailedExitCode(c) => {
            debug!(
                "Scheduling rerun for run #{} that failed with exit code {}",
                specific_run, c
            );
            Ok(*specific_run)
        }

        RerunStatus::FailedErrorLabel(l) => {
            debug!(
                "Scheduling rerun for run #{} that failed with label {}",
                specific_run, l
            );
            Ok(*specific_run)
        }
    }
}

/// Check the statuses of a list of runs and ask the user what to rerun.
pub(super) fn check_multiple_runs_failed(
    user_list: &[usize],
    experiment: &Experiment,
    statuses: &ExperimentStatus,
) -> Result<Vec<usize>> {
    let mut list = user_list.to_vec();
    list.dedup();
    if list.len() < RERUN_LIST_PROMPT_CUTOFF {
        Ok(list
            .iter()
            .filter_map(|r| check_single_run_failed(r, experiment, statuses).ok())
            .collect())
    } else {
        let failed = re_runnable(list.iter().copied(), experiment, statuses);

        let choices = vec!["Rerun only failed", "Rerun all", "Cancel"];
        match ask(Select::new(
            &format!(
                "There are {HELP_STYLE}{}{HELP_STYLE:#} failed runs \
                and {HELP_STYLE}{}{HELP_STYLE:#} total runs. What would you like to do?",
                failed.len(),
                list.len()
            ),
            choices,
        )
        .prompt())?
        {
            "Rerun only failed" => Ok(failed),
            "Rerun all" => Ok(list.to_vec()),
            "Cancel" => bailc!("Rerun cancelled", ; "", ; "",),
            _ => unreachable!(),
        }
    }
}

/// Query the user for the resource limits of the programs for a list of runs.
pub fn query_changing_limits_for_programs(
    runs: &[usize],
    experiment: &mut Experiment,
) -> Result<()> {
    loop {
        let mut run_programs = BTreeMap::new();

        for run in runs {
            run_programs.insert(
                *run,
                experiment.programs[experiment.runs[*run].program].clone(),
            );
        }

        let mut choices = run_programs
            .keys()
            .map(|p| {
                format!(
                    "{NAME_STYLE}{}{NAME_STYLE:#}",
                    experiment.runs[*p].program.clone()
                )
            })
            .collect::<Vec<String>>();
        choices.dedup();
        choices.push("Done".to_string());

        match ask(Select::new("Update resource limits?", choices).prompt())?.as_str() {
            "Done" => break,
            x => {
                let mut changed = vec![];
                for (run_id, program) in run_programs.iter() {
                    if x == format!(
                        "{NAME_STYLE}{}{NAME_STYLE:#}",
                        experiment.runs[*run_id].program
                    ) {
                        if changed.contains(&program) {
                            continue;
                        }
                        let new_rss = query_update_resource_limits(
                            &experiment.runs[*run_id].limits,
                            false,
                            None,
                            None,
                            None,
                        )?;

                        debug!("Updating resource limits for run {}", run_id);
                        trace!("Old resource limits: {:?}", program.limits);
                        trace!("New resource limits: {:?}", new_rss);

                        update_program_resource_limits(*run_id, experiment, new_rss)?;
                        changed.push(program);
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/checks.rs"]
mod checks;
