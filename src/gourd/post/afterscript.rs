use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::ctx;
use gourd_lib::experiment::Experiment;
use log::debug;
use log::trace;

use crate::resources::run_script;

/// Runs the afterscript on jobs that are completed and do not yet have an
/// afterscript output.
pub fn run_afterscript(run_id: usize, experiment: &Experiment) -> Result<()> {
    let run = &experiment.runs[run_id];
    let after_out_path = &run.afterscript_output_path;
    let res_path = run.output_path.clone();

    trace!("Checking afterscript for {run_id}");

    let after_output = after_out_path
        .clone()
        .ok_or(anyhow!("Could not get the afterscript information"))
        .with_context(ctx!(
            "Could not get the afterscript information", ;
            "",
        ))?;

    let afterscript = &experiment.programs[run.program]
        .afterscript
        .clone()
        .ok_or(anyhow!("Could not get the afterscript information"))
        .with_context(ctx!(
            "Could not get the afterscript information", ;
            "",
        ))?;

    debug!("Running afterscript for {run_id}");
    let exit_status =
        run_afterscript_for_run(afterscript, &res_path, &after_output, &run.work_dir)?;

    if !exit_status.success() {
        bailc!("Afterscript failed with exit code {}",
                exit_status
                    .code()
                    .ok_or(anyhow!("Status does not exist"))
                    .with_context(ctx!(
                        "Could not get the exit code of the execution", ;
                        "",
                    ))? ; "", ; "", );
    }

    Ok(())
}

/// Runs the afterscript on given jobs.
pub fn run_afterscript_for_run(
    after_path: &PathBuf,
    res_path: &PathBuf,
    out_path: &PathBuf,
    work_dir: &Path,
) -> Result<ExitStatus> {
    fs::metadata(res_path).with_context(ctx!(
        "Could not find the job result at {:?}", &res_path;
        "Check that the job result already exists",
    ))?;

    let args = vec![
        res_path.as_os_str().to_str().with_context(ctx!(
            "Could not turn {res_path:?} into a string", ;
            "",
        ))?,
        out_path.as_os_str().to_str().with_context(ctx!(
            "Could not turn {out_path:?} into a string", ;
            "",
        ))?,
    ];

    let exit_status = run_script(after_path, args, work_dir).with_context(ctx!(
        "Could not run the afterscript at {after_path:?} with job results at {res_path:?}", ;
        "Check that the afterscript is correct and job results exist at {:?}", res_path,
    ))?;

    Ok(exit_status)
}

#[cfg(test)]
#[cfg(unix)]
#[path = "tests/mod.rs"]
mod tests;
