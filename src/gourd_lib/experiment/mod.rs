pub mod inputs;
pub mod labels;
pub mod programs;
pub mod scheduling;

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

use crate::config::maps::InternalInputMap;
use crate::config::maps::InternalProgramMap;
use crate::config::Config;
use crate::config::Label;
use crate::config::ResourceLimits;
use crate::config::SlurmConfig;
use crate::config::UserInput;
use crate::config::UserProgram;
use crate::experiment::inputs::RunInput;
use crate::experiment::labels::Labels;
use crate::experiment::scheduling::RunStatus;
use crate::file_system::FileOperations;

pub type Executable = PathBuf;
pub type FieldRef = String;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InternalInput {
    pub name: String,
    pub input: Option<PathBuf>,
    pub arguments: Vec<String>,
    pub is_glob: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InternalProgram {
    pub name: FieldRef,
    pub binary: Executable,
    pub is_glob: bool,
    pub afterscript: Option<Executable>,
    pub limits: ResourceLimits,
    pub arguments: Vec<String>,
    pub runs_after: Option<FieldRef>,
}

/// Describes a matching between an algorithm and an input.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// The unique name of the program to run.
    pub program: FieldRef,

    /// The path to the file to pass into stdin
    pub input: RunInput,

    /// The execution status of this run.
    pub status: RunStatus,

    /// The path to the stderr output.
    pub err_path: PathBuf,

    /// The path to the stdout output.
    pub output_path: PathBuf,

    /// The path to the metrics file.
    pub metrics_path: PathBuf,

    /// The path to afterscript output, if there is an afterscript.
    pub afterscript_output_path: Option<PathBuf>,

    /// The working directory of this run.
    pub work_dir: PathBuf,

    /// Slurm job id, if ran on slurm
    pub slurm_id: Option<String>,

    /// Resource limits applied to this run
    pub limits: ResourceLimits,

    /// If this job has been rerun, a reference to the new one.
    pub rerun: Option<usize>,

    /// If there's a dependency, which (other) run to wait on
    pub depends: Option<usize>,
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
pub struct Experiment {
    /// The ID of this experiment.
    pub seq: usize,

    /// The time of creation of the experiment.
    pub creation_time: DateTime<Local>,

    /// The directory in which the contents of this experiment reside
    pub home: PathBuf,

    /// What to call as a [std::process::Command] to get the wrapper executable.
    pub wrapper: String,

    /// The inputs for the experiment.
    pub inputs: InternalInputMap,

    /// The programs for the experiment.
    pub programs: InternalProgramMap,

    // /// Chunks created for this experiment.
    // pub chunks: Vec<Chunk>,
    pub output_path: PathBuf,
    pub metrics_path: PathBuf,
    pub afterscript_output_path: PathBuf,

    /// Global resource limits that will apply to _newly created chunks_.
    pub resource_limits: Option<ResourceLimits>,

    /// Environment of the experiment
    pub env: Environment,

    /// Labels used in this experiment.
    pub labels: Labels,

    /// If running on a SLURM cluster, the job configurations.
    pub slurm: Option<SlurmConfig>,

    // last in the struct so that the lockfile has these at the bottom
    /// The pairings of program-input for this experiment.
    pub runs: Vec<Run>,
}

impl Experiment {
    /// Save the experiment to a file with its timestamp.
    pub fn save_to(&self, folder: &Path, fs: &impl FileOperations) -> Result<PathBuf> {
        let saving_path = folder.join(format!("{}.lock", self.seq));

        fs.try_write_toml(&saving_path, &self)?;

        Ok(saving_path)
    }

    /// Save the experiment
    pub fn save(&self, fs: &impl FileOperations) -> Result<PathBuf> {
        let saving_path = self.home.join(format!("{}.lock", self.seq));

        fs.try_write_toml(&saving_path, &self)?;

        Ok(saving_path)
    }

    /// Get the label by name.
    pub fn get_label(&self, name: &String) -> Result<Label> {
        self.labels
            .map
            .get(name)
            .cloned()
            .ok_or(anyhow!("Label not found"))
    }

    pub fn get_program(&self, run: &Run) -> Result<InternalProgram> {
        self.programs
            .get(&run.program)
            .cloned()
            .ok_or_else(|| anyhow!("Could not find program for run {:?}", &run))
    }
}
