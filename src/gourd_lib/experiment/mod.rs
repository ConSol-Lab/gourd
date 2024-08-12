use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use chrono::DateTime;
use chrono::Local;
use serde::Deserialize;
use serde::Serialize;

use crate::config::maps::InternalInputMap;
use crate::config::Label;
use crate::config::ResourceLimits;
use crate::config::SlurmConfig;
use crate::ctx;
use crate::experiment::labels::Labels;
use crate::file_system::FileOperations;

/// Dealing with [`UserInput`]s and [`InternalInput`]s
pub mod inputs;

/// Everything related to [`Label`]s
pub mod labels;

/// Dealing with [`UserProgram`]s and [`InternalProgram`]s
pub mod programs;

/// A string referencing a [`UserProgram`], [`InternalProgram`], [`UserInput`]
/// or [`InternalInput`].
pub type FieldRef = String;

/// The internal representation of a [`UserInput`]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InternalInput {
    /// A file to pass the contents into `stdin`
    pub input: Option<PathBuf>,

    /// Command line arguments to be passed to the executable
    pub arguments: Vec<String>,

    #[allow(dead_code)]
    /// Additional data for this input
    pub metadata: Metadata,
}

/// Internally used metadata for inputs/programs
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Metadata {
    /// Which input this was generated from.
    pub glob_from: Option<String>,

    /// Whether it was fetched.
    pub is_fetched: bool,
}

/// The internal representation of a [`UserProgram`]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct InternalProgram {
    /// The name given to this program by the user.
    pub name: String,

    /// The [`Executable`] of this program (absolute path to it)
    pub binary: PathBuf,

    /// An executable afterscript to run on the output of this program
    pub afterscript: Option<PathBuf>,

    /// The limits to be applied on executions of this program
    pub limits: ResourceLimits,

    /// The command line arguments to be passed to all executions of this
    /// program
    pub arguments: Vec<String>,

    /// This program runs on the output of our program,
    /// a reference to the other program's name.
    pub next: Vec<usize>,
}

/// The input for a [`Run`], exactly as will be passed to the wrapper for
/// execution.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RunInput {
    /// A file whose contents to be passed into the program's `stdin`
    pub file: Option<PathBuf>,

    /// Command line arguments for this binary execution.
    ///
    /// Holds the concatenation of [`UserProgram`] specified arguments and
    /// [`UserInput`] arguments.
    pub arguments: Vec<String>,
}

/// Describes a matching between an algorithm and an input.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// The unique name of the program to run.
    pub program: usize,

    /// The path to the file to pass into stdin
    pub input: RunInput,

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

    /// The input this has been generated from.
    pub generated_from_input: Option<FieldRef>,

    /// Edge to the parent run.
    pub parent: Option<usize>,
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
    pub programs: Vec<InternalProgram>,

    /// The path to a folder where the experiment output will be stored.
    pub output_folder: PathBuf,

    /// The path to a folder where the metrics output will be stored.
    pub metrics_folder: PathBuf,

    /// The path to a folder where the afterscript output will be stored.
    pub afterscript_output_folder: PathBuf,

    /// Global resource limits that will apply to _newly created chunks_.
    pub resource_limits: Option<ResourceLimits>,

    /// Environment of the experiment
    pub env: Environment,

    /// Labels used in this experiment.
    pub labels: Labels,

    /// If running on a SLURM cluster, the job configurations.
    pub slurm: Option<SlurmConfig>,

    /// A mapping of job array task id indices to run ids.
    pub chunks: Vec<Vec<usize>>,

    // last in the struct so that the lockfile has these at the bottom
    /// The pairings of program-input for this experiment.
    pub runs: Vec<Run>,
}

impl Experiment {
    /// Path to the experiment lockfile.
    pub fn file(&self) -> PathBuf {
        self.home.join(format!("{}.lock", self.seq))
    }

    /// Save the experiment to a file with its timestamp.
    pub fn save_to(&self, folder: &Path, fs: &impl FileOperations) -> Result<PathBuf> {
        let saving_path = folder.join(format!("{}.lock", self.seq));

        fs.try_write_toml(&saving_path, &self)?;

        Ok(saving_path)
    }

    /// Save the experiment
    pub fn save(&self, fs: &impl FileOperations) -> Result<PathBuf> {
        let saving_path = self.file();

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

    /// Get (a clone of) the [`InternalProgram`] used for a given [`Run`].
    pub fn get_program(&self, run: &Run) -> Result<InternalProgram> {
        self.programs
            .get(run.program)
            .cloned()
            .ok_or_else(|| anyhow!("Could not find program for run {:?}", &run))
            .with_context(ctx!("",;"",))
    }

    /// Get (a clone of) the [`InternalProgram`] used for a given [`Run`].
    pub fn program_from_run_id(&self, run_id: usize) -> Result<InternalProgram> {
        let run = self
            .runs
            .get(run_id)
            .ok_or_else(|| anyhow!("Could not find run {}", run_id))
            .with_context(ctx!("",;"",))?;

        self.programs
            .get(run.program)
            .cloned()
            .ok_or_else(|| anyhow!("Could not find program for run {:?}", &run))
            .with_context(ctx!("",;"",))
    }

    /// Get the slurm stdout file path for a given run.
    pub fn slurm_out(&self, slurm_id: &str) -> Option<PathBuf> {
        self.slurm
            .as_ref()
            .map(|opt| opt.output_folder.join(format!("gourd_{}.out", slurm_id)))
    }

    /// Get the slurm stderr file path for a given run.
    pub fn slurm_err(&self, slurm_id: &str) -> Option<PathBuf> {
        self.slurm
            .as_ref()
            .map(|opt| opt.output_folder.join(format!("gourd_{}.err", slurm_id)))
    }
}
