use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use chrono::DateTime;
use chrono::Local;
use gourd_lib::bailc;
use gourd_lib::config::Config;
use gourd_lib::ctx;
use gourd_lib::experiment::inputs::expand_inputs;
use gourd_lib::experiment::programs::expand_programs;
use gourd_lib::experiment::Environment;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;

use crate::experiments::dfs::dfs;

/// Search through the run dependency graph to create the linear-connected runs
mod dfs;

/// Generating new runs
pub mod run;

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

        // First we will explode all programs from the initial set to their final set.
        let expanded_programs = expand_programs(&conf.programs, conf, fs)?;

        // Now we will expand all inputs in a similar manner.
        let expanded_inputs = expand_inputs(&conf.inputs, &conf.parameters, fs)?;

        // Modifications to the slurm configurations
        let slurm = if let Some(mut slurm_conf) = conf.slurm.clone() {
            // NOTE: if not all directories exist, slurm will fail with no obvious reason
            // why.
            slurm_conf.output_folder = fs.truncate_and_canonicalize_folder(&conf.output_path)?;
            // ...
            Some(slurm_conf)
        } else {
            None
        };

        // get the groups from inputs
        let groups = expanded_inputs
            .iter()
            .filter_map(|(_, input)| input.metadata.group.clone())
            .collect();

        let mut experiment = Self {
            seq,
            creation_time: time,
            home: fs.truncate_and_canonicalize_folder(&conf.experiments_folder)?,
            wrapper: conf.wrapper.clone(),

            inputs: expanded_inputs.clone(),
            programs: expanded_programs,

            output_folder: fs.truncate_and_canonicalize_folder(&conf.output_path)?,
            metrics_folder: fs.truncate_and_canonicalize_folder(&conf.metrics_path)?,
            afterscript_output_folder: fs
                .truncate_and_canonicalize_folder(&conf.output_path.join("afterscripts"))?,

            env,
            resource_limits: conf.resource_limits,
            labels: conf.labels.clone().unwrap_or_default(),

            slurm,

            chunks: Vec::new(),
            groups,
            runs: Vec::new(),
        };

        // Collapse the programs into an ordered structure for faster processing.
        let mut in_degrees = vec![0usize; experiment.programs.len()];

        for prog in &experiment.programs {
            for next_prog in &prog.next {
                in_degrees[*next_prog] += 1;
            }
        }

        let mut visitation = vec![0usize; experiment.programs.len()];
        let mut runs = Vec::new();

        for (prog, degree) in in_degrees.iter().enumerate() {
            if *degree == 0 {
                dfs(&mut visitation, prog, &mut runs, &experiment, fs)?;
            }
        }

        for (prog, visit) in visitation.iter().enumerate() {
            if *visit != 1 {
                bailc!(
                    "A cycle was found in the program dependencies.",;
                    "The `next` field in the program definitions created a circular dependency",;
                    "Fix the dependencies for {:?}",experiment.programs[prog].name
                );
            }
        }

        experiment.runs = runs;

        Ok(experiment)
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

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
