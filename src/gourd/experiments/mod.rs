use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use chrono::DateTime;
use chrono::Local;
use gourd_lib::config::Config;
use gourd_lib::ctx;
use gourd_lib::error::Ctx;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::Run;
use gourd_lib::experiment::SlurmExperiment;
use gourd_lib::file_system::FileOperations;

use crate::cli::process::Environment;

/// Extension trait for the shared `Experiment` struct.
pub trait ExperimentExt {
    /// Initialize a new experiment from a `config`.
    ///
    /// Creates a new experiment by matching all algorithms to all inputs.
    /// The experiment is created in the provided `env` and with `time` as the timestamp.
    fn from_config(
        conf: &Config,
        env: Environment,
        time: DateTime<Local>,
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
        env: Environment,
        time: DateTime<Local>,
        fs: &impl FileOperations,
    ) -> Result<Self> {
        let mut runs = Vec::new();

        let seq = Self::latest_id_from_folder(&conf.experiments_folder)
            .unwrap_or(Some(0))
            .unwrap_or(0)
            + 1;

        let slurm: Option<SlurmExperiment> = match env {
            Environment::Slurm => {
                if conf.slurm.is_none() {
                    return Err(anyhow!(
                        "A SLURM configuration missing from this config file."
                    ))
                    .with_context(ctx!("", ;
                        "Fill the field `slurm` in your gourd.toml if you want to run on SLURM",));
                }

                let limits = conf
                    .resource_limits
                    .clone()
                    .map(|lim| SlurmExperiment {
                        chunks: vec![],
                        resource_limits: lim.clone(),
                    })
                    .ok_or(anyhow!(
                        "SLURM resource limits are missing from this config file."
                    ))
                    .with_context(ctx!(
                        "",;
                        "Add `resource_limits` to your gourd.toml if you want to run on SLURM",
                    ))?;

                Some(limits)
            }

            Environment::Local => None,
        };

        for prog_name in conf.programs.keys() {
            for input_name in conf.inputs.keys() {
                runs.push(Run {
                    program: prog_name.clone(),
                    input: input_name.clone(),
                    err_path: fs.truncate_and_canonicalize(
                        &conf
                            .output_path
                            .join(format!("{}/{}/{}_error", seq, prog_name, input_name)),
                    )?,
                    metrics_path: fs.truncate_and_canonicalize(
                        &conf
                            .metrics_path
                            .join(format!("{}/{}/{}_metrics", seq, prog_name, input_name)),
                    )?,
                    output_path: fs.truncate_and_canonicalize(
                        &conf
                            .output_path
                            .join(format!("{}/{}/{}_output", seq, prog_name, input_name)),
                    )?,
                    afterscript_output_path: conf.afterscript_output_folder.as_ref().map(
                        |out_path| {
                            out_path
                                .join(format!("{}/{}/afterscript_{}", seq, prog_name, input_name))
                        },
                    ),
                    post_job_output_path: get_postprocess_job_info(
                        // todo for ruta: check that this is correct
                        conf, &seq, prog_name, input_name, fs,
                    )?,
                    job_id: None,
                });
            }
        }

        Ok(Self {
            runs,
            slurm,
            creation_time: time,
            seq,
            config: conf.clone(),
        })
    }

    fn latest_id_from_folder(folder: &Path) -> Result<Option<usize>> {
        let mut highest = None;

        for file in fs::read_dir(folder).with_context(ctx!(
          "Could not access the experiments directory {folder:?}", ;
          "Run some experiments first or ensure that you have suffient permissions to read it",
        ))? {
            let actual = match file {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let seq_of_file: usize = actual
                .file_name()
                .to_str()
                .ok_or(anyhow!("Invalid filename in experiment directory"))?
                .trim_end_matches(".lock")
                .parse()
                .with_context(ctx!(
                  "Invalid name of experiment file {actual:?}", ;
                  "Do not manually modify files in the experiment directory",
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

    fn experiment_from_folder(
        seq: usize,
        folder: &Path,
        fs: &impl FileOperations,
    ) -> Result<Experiment> {
        fs.try_read_toml(&folder.join(format!("{}.lock", seq)))
    }

    fn latest_experiment_from_folder(
        folder: &Path,
        fs: &impl FileOperations,
    ) -> Result<Experiment> {
        if let Some(id) = Self::latest_id_from_folder(folder)? {
            Self::experiment_from_folder(id, folder, fs)
        } else {
            Err(anyhow!("There are no experiments, try running some first").context(""))
        }
    }
}

/// Constructs an afterscript path based on values in the config.
pub fn get_afterscript_info(
    config: &Config,
    seq: &usize,
    prog_name: &String,
    input_name: &String,
    fs: &impl FileOperations,
) -> Result<Option<PathBuf>> {
    let postprocessing = &config.programs[prog_name].afterscript;

    if let Some(path) = postprocessing {
        let afterscript_output_path = fs.truncate_and_canonicalize(&path.clone().join(format!(
            "{}/algo_{}/afterscript_{}",
            seq, prog_name, input_name
        )))?;

        Ok(Some(afterscript_output_path))
    } else {
        Ok(None)
    }
}

/// Constructs a postprocess job output path based on values in the config.
pub fn get_postprocess_job_info(
    config: &Config,
    seq: &usize,
    prog_name: &String,
    input_name: &String,
    fs: &impl FileOperations,
) -> Result<Option<PathBuf>> {
    let postprocessing = &config.programs[prog_name].postprocess_job;

    if let Some(path) = postprocessing {
        let postprocess_output_path = fs.truncate_and_canonicalize(&path.clone().join(format!(
            "{}/algo_{}postprocess_job_{}",
            seq, prog_name, input_name
        )))?;

        Ok(Some(postprocess_output_path))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
