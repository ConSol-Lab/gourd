use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use chrono::DateTime;
use chrono::Local;
use gourd_lib::bailc;
use gourd_lib::config::Config;
use gourd_lib::config::ResourceLimits;
use gourd_lib::ctx;
use gourd_lib::experiment::inputs::expand_inputs;
use gourd_lib::experiment::inputs::RunInput;
use gourd_lib::experiment::labels::Labels;
use gourd_lib::experiment::programs::expand_programs;
use gourd_lib::experiment::programs::topological_ordering;
use gourd_lib::experiment::scheduling::RunStatus;
use gourd_lib::experiment::Environment;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::InternalProgram;
use gourd_lib::experiment::Run;
use gourd_lib::file_system::FileOperations;

/// Extension trait for the shared `Experiment` struct.
pub trait ExperimentExt {
    /// Initialize a new experiment from a `config`.
    ///
    /// Creates a new experiment by matching all algorithms to all inputs.
    /// The experiment is created in the provided `env` and with `time` as the
    /// timestamp.
    fn from_config(
        conf: &Config,
        time: DateTime<Local>,
        env: Environment,
        fs: &impl FileOperations,
    ) -> Result<Self>
    where
        Self: Sized;

    /// Get the filename of the newest experiment.
    fn latest_id_from_folder(folder: &Path) -> Result<Option<usize>>;

    /// Provided a folder gets the most recent experiment.
    fn latest_experiment_from_folder(folder: &Path, fs: &impl FileOperations)
        -> Result<Experiment>;

    /// Provided a folder gets the specified experiment.
    fn experiment_from_folder(
        seq: usize,
        folder: &Path,
        fs: &impl FileOperations,
    ) -> Result<Experiment>;
}

impl ExperimentExt for Experiment {
    fn from_config(
        conf: &Config,
        time: DateTime<Local>,
        env: Environment,
        fs: &impl FileOperations,
    ) -> Result<Self> {
        let seq = Self::latest_id_from_folder(&conf.experiments_folder)
            .unwrap_or(Some(0))
            .unwrap_or(0)
            + 1;

        let expanded_programs = expand_programs(&conf.programs, conf, fs)?;
        let expanded_inputs = expand_inputs(&conf.inputs, &conf.parameters, fs)?;

        let sorted_programs: Vec<String> = topological_ordering(&expanded_programs)?
            .into_iter()
            .map(|(x, _)| x)
            .collect();

        let mut e = Self {
            runs: Vec::new(),
            creation_time: time,
            home: conf.experiments_folder.clone(),
            wrapper: conf.wrapper.clone(),
            inputs: expanded_inputs.clone(),
            seq,
            resource_limits: conf.resource_limits,
            env,
            labels: Labels {
                map: conf.labels.clone().unwrap_or_default(),
                warn_on_label_overlap: conf.warn_on_label_overlap,
            },
            programs: expanded_programs,
            output_folder: conf.output_path.clone(),
            metrics_folder: conf.metrics_path.clone(),
            slurm: conf.slurm.clone(),
            afterscript_output_folder: Default::default(), //todo
        };

        for prog_name in &sorted_programs {
            for input_name in expanded_inputs.keys() {
                if let Some(parent) = &e.programs[prog_name].runs_after {
                    for (i, prev) in e.runs.clone().iter().enumerate() {
                        if prev.program.eq(parent) {
                            e.runs.push(generate_new_run(
                                e.runs.len(),
                                &e.programs[prog_name],
                                RunInput {
                                    name: format!("output_of_run_{}", i),
                                    file: Some(prev.output_path.clone()),
                                    args: e.programs[prog_name].arguments.clone(),
                                },
                                Some(i),
                                e.programs[prog_name].limits,
                                &e,
                                fs,
                            )?);
                        }
                    }
                } else {
                    e.runs.push(generate_new_run(
                        e.runs.len(),
                        &e.programs[prog_name],
                        RunInput {
                            name: input_name.clone(),
                            file: e.inputs[input_name].input.clone(),
                            args: [
                                e.programs[prog_name].arguments.clone(),
                                e.inputs[input_name].arguments.clone(),
                            ]
                            .concat(),
                        },
                        None,
                        e.programs[prog_name].limits,
                        &e,
                        fs,
                    )?);
                }
            }
        }

        Ok(e)
    }

    fn latest_id_from_folder(folder: &Path) -> Result<Option<usize>> {
        let mut highest = None;

        // More lenient handling of other files in the experiment directory,
        // as long as they are not *.lock files.
        // This is in line with some of our examples, which have been adjusted
        // to also put output and metrics in the experiment dir for cleanness.
        let file_names = fs::read_dir(folder).with_context(ctx!(
          "Could not access the experiments directory {folder:?}", ;
          "Run some experiments first or ensure that you have sufficient permissions to read it",
        ))?.flatten()
            // get only regular files
            .filter(|f| f.file_type().is_ok_and(|fty| fty.is_file()))
            // get file names
            .filter_map(|f| f.file_name().into_string().ok())
            // get only .lock files
            .filter(|name| name.ends_with(".lock"));

        for file_name in file_names {
            let seq_of_file = file_name
                .trim_end_matches(".lock")
                .parse()
                .with_context(ctx!(
                  "Invalid name of experiment file {:?}", file_name;
                  "Do not manually modify .lock files in the experiment directory",
                ))?;

            if highest.is_none() {
                highest = Some(seq_of_file);
            } else if let Some(num) = highest {
                if seq_of_file > num {
                    highest = Some(seq_of_file);
                }
            }
        }

        Ok(highest)
    }

    fn latest_experiment_from_folder(
        folder: &Path,
        fs: &impl FileOperations,
    ) -> Result<Experiment> {
        if let Some(id) = Self::latest_id_from_folder(folder)? {
            Self::experiment_from_folder(id, folder, fs)
        } else {
            bailc!(
                "There are no experiments, try running some first", ;
                "", ;
                "",
            );
        }
    }

    fn experiment_from_folder(
        seq: usize,
        folder: &Path,
        fs: &impl FileOperations,
    ) -> Result<Experiment> {
        fs.try_read_toml(&folder.join(format!("{}.lock", seq)))
    }
}

/// This function will generate a new run.
///
/// This should be used by all code paths adding runs to the experiment.
pub fn generate_new_run(
    run_id: usize,
    program: &InternalProgram,
    input: RunInput,
    depends: Option<usize>,
    limits: ResourceLimits,
    experiment: &Experiment,
    fs: &impl FileOperations,
) -> Result<Run> {
    Ok(Run {
        program: program.name.clone(),
        input,
        status: RunStatus::Pending,
        err_path: fs.truncate_and_canonicalize(&experiment.output_folder.join(format!(
            "{}/{}/{}/stderr",
            experiment.seq, program.name, run_id
        )))?,
        metrics_path: fs.truncate_and_canonicalize(&experiment.metrics_folder.join(format!(
            "{}/{}/{}/metrics",
            experiment.seq, program.name, run_id
        )))?,
        output_path: fs.truncate_and_canonicalize(&experiment.output_folder.join(format!(
            "{}/{}/{}/stdout",
            experiment.seq, program.name, run_id
        )))?,
        work_dir: fs.truncate_and_canonicalize_folder(
            &experiment
                .output_folder
                .join(format!("{}/{}/{}/", experiment.seq, program.name, run_id)),
        )?,
        afterscript_output_path: match program.afterscript.as_ref() {
            None => None,
            Some(_) => Some(
                fs.truncate_and_canonicalize_folder(
                    &experiment
                        .output_folder
                        .join(format!("{}/{}/{}/", experiment.seq, program.name, run_id)),
                )?,
            ),
        },
        limits,
        depends,
        slurm_id: None,
        rerun: None,
    })
}

// #[cfg(test)]
// #[path = "tests/mod.rs"]
// mod tests;
