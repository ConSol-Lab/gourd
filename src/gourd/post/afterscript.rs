use anyhow::anyhow;
use anyhow::Result;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use gourd_lib::resources::run_script;
use log::debug;
use log::trace;

/// For a run that:
/// * has finished
/// * its program has an afterscript
///
/// this function will run said afterscript, and update the experiment
/// accordingly.
pub fn run_afterscript(run_id: usize, experiment: &mut Experiment) -> Result<()> {
    let run = &experiment.runs[run_id];
    let run_output_path = run.output_path.clone();

    trace!("Checking afterscript for {run_id}");

    let afterscript = &experiment.programs[run.program]
        .afterscript
        .clone()
        .ok_or(anyhow!("Could not get the afterscript information"))?;

    debug!("Running afterscript for {run_id}");
    let afterscript_output = run_script(
        &afterscript.executable,
        vec![&run_output_path.display().to_string()],
        &run.work_dir,
    )?;

    let afterscript_result = String::from_utf8_lossy(&afterscript_output.stdout)
        .trim()
        .to_string();
    debug!("stdout: {afterscript_result}");
    debug!(
        "stderr: {}",
        String::from_utf8_lossy(&afterscript_output.stdout).trim()
    );

    experiment.runs[run_id].afterscript_output = Some(afterscript_result);

    Ok(())
}

/// Run all the afterscripts that haven't been run yet for this experiment
///
/// checks that the afterscript exists and hasn't already ran.
pub fn run_afterscripts_for_experiment(
    experiment: &mut Experiment,
    fs: &impl FileOperations,
) -> Result<()> {
    for run_id in 0..experiment.runs.len() {
        if experiment.runs[run_id].afterscript_output.is_none()
            && experiment
                .get_program(&experiment.runs[run_id])?
                .afterscript
                .is_some()
        {
            run_afterscript(run_id, experiment)?;
        }
    }

    experiment.save(fs)?;

    Ok(())
}

#[cfg(test)]
#[cfg(unix)]
#[path = "tests/mod.rs"]
mod tests;
