use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::ctx;
use gourd_lib::error::Ctx;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use inquire::Select;

use crate::rerun::checks::check_multiple_runs_failed;
use crate::status::get_statuses;
use crate::status::ExperimentStatus;

/// Get the list of runs to rerun from the rerun options.
pub fn get_runs_from_rerun_options(
    run_ids: &Option<Vec<usize>>,
    experiment: &Experiment,
    file_system: &mut impl FileOperations,
    script: bool,
) -> Result<Vec<usize>> {
    let statuses = get_statuses(experiment, file_system)?;
    if let Some(runs) = run_ids {
        for id in runs {
            if experiment
                .runs
                .get(*id)
                .ok_or(anyhow!("Run {id} does not exist"))
                .with_context(ctx!(
                    "", ;
                    "You can only rerun runs in the range 0-{}", experiment.runs.len(),
                ))?
                .rerun
                .is_some()
            {
                bailc!(
                    "Cannot rerun run {id}", ;
                    "", ;
                    "You cannot rerun runs which have been already rerun",
                );
            }
        }

        if script {
            Ok(runs.clone())
        } else {
            Ok(check_multiple_runs_failed(runs, experiment, &statuses)?)
        }
    } else {
        Ok(get_what_runs_to_rerun_from_experiment(
            experiment, statuses, script,
        )?)
    }
}

/// Get the list of runs that have finished and ask the user what to rerun.
pub(super) fn get_what_runs_to_rerun_from_experiment(
    experiment: &Experiment,
    statuses: ExperimentStatus,
    script: bool,
) -> Result<Vec<usize>> {
    let all_runs = 0..experiment.runs.len();

    let all_not_rerun: Vec<usize> = all_runs
        .filter(|id| experiment.runs[*id].rerun.is_none() && statuses[id].is_completed())
        .collect();

    let failed_runs: Vec<usize> = all_not_rerun
        .clone()
        .into_iter()
        .filter(|id| statuses[id].has_failed())
        .collect();

    if script {
        return Ok(failed_runs);
    }

    let choices: Vec<String> = vec![
        format!("Rerun only failed ({} runs)", failed_runs.len()),
        format!("Rerun all finished ({} runs)", all_not_rerun.len()),
    ];
    match Select::new("What would you like to do?", choices.clone())
        .prompt()
        .with_context(ctx!("",;"",))?
        .as_str()
    {
        x if x == choices[1] => Ok::<Vec<usize>, anyhow::Error>(all_not_rerun),
        x if x == choices[0] => Ok::<Vec<usize>, anyhow::Error>(failed_runs),
        x => unreachable!("got: {:?}", x),
    }
}
