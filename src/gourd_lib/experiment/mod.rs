pub mod chunks;
mod labels;

use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use chrono::DateTime;
use chrono::Local;
use serde::Deserialize;
use serde::Serialize;

use crate::config::Config;
use crate::config::Input;
use crate::config::Label;
use crate::config::Program;
use crate::config::ResourceLimits;
use crate::experiment::chunks::Chunk;
use crate::experiment::labels::Labels;
use crate::file_system::FileOperations;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InternalInput {
    pub name: String,
    pub input: PathBuf,
    pub is_glob: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InternalProgram<'exp> {
    pub name: String,
    pub binary: PathBuf,
    pub is_glob: bool,
    pub runs_after: Option<&'exp InternalProgram<'exp>>
}

/// Describes a matching between an algorithm and an input.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Run<'exp> {
    /// The unique name of the program to run.
    pub program: &'exp InternalProgram<'exp>,

    /// The unique name of the input to run with.
    pub input: &'exp InternalInput,

    /// The path to the stderr output.
    pub err_path: PathBuf,

    /// The path to the stdout output.
    pub output_path: PathBuf,

    /// The path to the metrics file.
    pub metrics_path: PathBuf,

    /// The working directory of this run.
    pub work_dir: PathBuf,

    /// Slurm job id, if ran on slurm
    pub slurm_id: Option<String>,

    /// The path to afterscript output, optionally.
    pub afterscript_output_path: Option<PathBuf>,

    /// The path to postprocess job output, if there is one.
    pub postprocessor: Option<usize>,

    /// If this job has been rerun, a reference to the new one.
    pub rerun: Option<usize>,
}

/// An enum to distinguish the run context.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Environment {
    /// Local execution.
    Local,

    /// Slurm execution.
    Slurm,
}

/// Describes one experiment.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Experiment<'exp> {
    /// The ID of this experiment.
    pub seq: usize,

    /// The time of creation of the experiment.
    pub creation_time: DateTime<Local>,

    /// The inputs for the experiment.
    pub inputs: BTreeMap<String, InternalInput>,

    /// The programs for the experiment.
    pub programs: BTreeMap<String, InternalProgram<'exp>>,

    /// Chunks created for this experiment.
    pub chunks: Vec<Chunk>,

    /// Global resource limits that will apply to _newly created chunks_.
    pub resource_limits: Option<ResourceLimits>,

    /// Environment of the experiment
    pub env: Environment,

    /// Labels used in this experiment.
    pub labels: Labels,

    // last in the struct so that the lockfile has these at the bottom
    /// The pairings of program-input for this experiment.
    pub runs: Vec<Run<'exp>>,
}

impl Experiment {
    /// Save the experiment to a file with its timestamp.
    pub fn save(&self, folder: &Path, fs: &impl FileOperations) -> Result<PathBuf> {
        let saving_path = folder.join(format!("{}.lock", self.seq));

        fs.try_write_toml(&saving_path, &self)?;

        Ok(saving_path)
    }

    /// Get the label by name.
    pub fn get_label(&self, name: &String) -> Result<Label> {
        self
            .labels
            .labels
            .get(name).cloned()
            .ok_or(anyhow!("Label not found"))
    }
}
