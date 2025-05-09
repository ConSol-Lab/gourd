use anyhow::Result;
use gourd_lib::config::slurm::ResourceLimits;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::FieldRef;
use gourd_lib::experiment::Run;
use gourd_lib::experiment::RunInput;
use gourd_lib::file_system::FileOperations;
/// This function will generate a new run.
///
/// This should be used by all code paths adding runs to the experiment.
/// This does *not* set the parent and child.
#[allow(clippy::too_many_arguments)]
pub fn generate_new_run(
    run_id: usize,
    program: usize,
    run_input: RunInput,
    input: Option<FieldRef>,
    input_group: Option<String>,
    limits: ResourceLimits,
    parent: Option<usize>,
    experiment: &Experiment,
    fs: &impl FileOperations,
) -> Result<Run> {
    let seq = experiment.seq;
    Ok(Run {
        program,
        input: run_input,
        err_path: fs.truncate_and_canonicalize(
            &experiment
                .output_folder
                .join(format!("{seq}/{program}/{run_id}/stderr")),
        )?,
        metrics_path: fs.truncate_and_canonicalize(
            &experiment
                .metrics_folder
                .join(format!("{seq}/{program}/{run_id}/metrics")),
        )?,
        output_path: fs.truncate_and_canonicalize(
            &experiment
                .output_folder
                .join(format!("{seq}/{program}/{run_id}/stdout")),
        )?,
        work_dir: fs.truncate_and_canonicalize_folder(
            &experiment
                .output_folder
                .join(format!("{seq}/{program}/{run_id}/")),
        )?,
        afterscript_output: None,
        limits,
        slurm_id: None,
        rerun: None,
        generated_from_input: input,
        parent,
        group: input_group,
    })
}
