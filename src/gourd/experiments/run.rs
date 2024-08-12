use gourd_lib::config::ResourceLimits;
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
    limits: ResourceLimits,
    parent: Option<usize>,
    experiment: &Experiment,
    fs: &impl FileOperations,
) -> anyhow::Result<Run> {
    Ok(Run {
        program,
        input: run_input,
        err_path: fs.truncate_and_canonicalize(
            &experiment
                .output_folder
                .join(format!("{}/{}/{}/stderr", experiment.seq, program, run_id)),
        )?,
        metrics_path: fs.truncate_and_canonicalize(
            &experiment
                .metrics_folder
                .join(format!("{}/{}/{}/metrics", experiment.seq, program, run_id)),
        )?,
        output_path: fs.truncate_and_canonicalize(
            &experiment
                .output_folder
                .join(format!("{}/{}/{}/stdout", experiment.seq, program, run_id)),
        )?,
        work_dir: fs.truncate_and_canonicalize_folder(
            &experiment
                .output_folder
                .join(format!("{}/{}/{}/", experiment.seq, program, run_id)),
        )?,
        afterscript_output_path: match experiment.programs[program].afterscript.as_ref() {
            None => None,
            Some(_) => Some(
                fs.truncate_and_canonicalize_folder(
                    &experiment
                        .output_folder
                        .join(format!("{}/{}/{}/", experiment.seq, program, run_id)),
                )?,
            ),
        },
        limits,
        slurm_id: None,
        rerun: None,
        generated_from_input: input,
        parent,
    })
}
