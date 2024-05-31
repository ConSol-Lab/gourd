use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use gourd_lib::config::Input;
use gourd_lib::ctx;
use gourd_lib::error::Ctx;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::Run;
use gourd_lib::file_system::FileOperations;

use crate::status::ExperimentStatus;
use crate::status::PostprocessCompletion;
use crate::status::SlurmState;

/// Schedules the postprocessing job for jobs that are completed and do not yet have a postprocess job output.
pub fn schedule_post_jobs(
    experiment: &mut Experiment,
    statuses: &mut ExperimentStatus,
    fs: &impl FileOperations,
) -> Result<()> {
    let runs = filter_runs_for_post_job(statuses)?;
    let _length = runs.len();

    for run_id in runs {
        let run = &experiment.runs[*run_id];
        let post_out_path = &run.post_job_output_path;
        let res_path = run.output_path.clone();

        if post_out_path.is_none() {
            continue;
        }

        let post_output = post_out_path
            .clone()
            .ok_or(anyhow!("Could not get the postprocessing information"))
            .with_context(ctx!(
                "Could not get the postprocessing information", ;
                "",
            ))?;

        let program = &experiment.config.programs[&run.program];

        let postprocess = program
            .postprocess_job
            .clone()
            .ok_or(anyhow!("Could not get the postprocessing information"))
            .with_context(ctx!(
                "Could not get the postprocessing information", ;
                "",
            ))?;

        post_job_for_run(
            format!("{}_{}", run.program, run.input),
            run.program.clone(),
            postprocess,
            &res_path,
            &post_output,
            experiment,
            fs,
        )?
    }

    Ok(())
}

/// Finds the completed jobs where posprocess job did not run yet.
pub fn filter_runs_for_post_job(runs: &mut ExperimentStatus) -> Result<Vec<&usize>> {
    let mut filtered = vec![];

    for (run_id, status) in runs {
        if status.slurm_status.is_some() {
            if let (SlurmState::Success, Some(PostprocessCompletion::Dormant)) = (
                &status.slurm_status.unwrap().completion,
                &status.fs_status.postprocess_job_completion,
            ) {
                filtered.push(run_id);
            }
        }
    }

    Ok(filtered)
}

/// Schedules the postprocess job for given jobs.
pub fn post_job_for_run(
    _name: String,
    original_name: String,
    postprocess_name: String,
    postprocess_input: &PathBuf,
    postprocess_out: &Path,
    experiment: &mut Experiment,
    fs: &impl FileOperations,
) -> Result<()> {
    // let prog_name = format!("{}{}", INTERNAL_POST, name);
    // let input_name = format!("{}{}", INTERNAL_POST, name);

    let input_name = original_name.clone() + "_output_as_input_for" + postprocess_name.as_str();

    experiment.config.inputs.insert(
        input_name.clone(),
        Input {
            input: Some(postprocess_input.clone()),
            arguments: vec![],
        },
    );

    experiment.runs.push(Run {
        program: postprocess_name,
        input: input_name,
        err_path: fs.truncate_and_canonicalize(
            &postprocess_out.join(format!("error_{:?}", postprocess_input)),
        )?,
        metrics_path: fs.truncate_and_canonicalize(
            &experiment
                .config
                .metrics_path
                .join(format!("metrics_{:?}", postprocess_input)),
        )?,
        output_path: fs.truncate_and_canonicalize(
            &postprocess_out.join(format!("output_{:?}", postprocess_input)),
        )?,
        afterscript_output_path: None,
        post_job_output_path: None, // these two can be updated to allow pipelining
        slurm_id: None,
    });

    Ok(())
}
