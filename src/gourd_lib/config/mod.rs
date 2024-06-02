use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::AFTERSCRIPT_DEFAULT;
use crate::constants::AFTERSCRIPT_OUTPUT_DEFAULT;
use crate::constants::EMPTY_ARGS;
use crate::constants::INPUTS_DEFAULT;
use crate::constants::INTERNAL_PREFIX;
use crate::constants::INTERNAL_SCHEMA_INPUTS;
use crate::constants::POSTPROCESS_JOBS_DEFAULT;
use crate::constants::POSTPROCESS_JOB_DEFAULT;
use crate::constants::POSTPROCESS_JOB_OUTPUT_DEFAULT;
use crate::constants::PRIMARY_STYLE;
use crate::constants::PROGRAM_RESOURCES_DEFAULT;
use crate::constants::RERUN_LABEL_BY_DEFAULT;
use crate::constants::WRAPPER_DEFAULT;
use crate::error::ctx;
use crate::error::Ctx;
use crate::file_system::FileOperations;

/// Deserializer for the duration.
mod duration;

/// Deserializer for the maps.
mod maps;

/// Deserializer for the regexes.
mod regex;

pub use maps::InputMap;
pub use maps::ProgramMap;
pub use regex::Regex;

use self::maps::UserDeserializer;

/// A pair of a path to a binary and cli arguments.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
pub struct Program {
    /// The path to the executable.
    pub binary: PathBuf,

    /// The cli arguments for the executable.
    #[serde(default = "EMPTY_ARGS")]
    pub arguments: Vec<String>,

    /// The path to the afterscript, if there is one.
    #[serde(default = "AFTERSCRIPT_DEFAULT")]
    pub afterscript: Option<PathBuf>,

    /// The path to the postprocess job, if there is one.
    #[serde(default = "POSTPROCESS_JOB_DEFAULT")]
    pub postprocess_job: Option<String>,

    /// Resource limits to optionally overwrite default resource limits.
    #[serde(default = "PROGRAM_RESOURCES_DEFAULT")]
    pub resource_limits: Option<ResourceLimits>,
}

/// A pair of a path to an input and additional cli arguments.
///
/// # Examples
///
/// ```toml
/// [programs.test_program]
/// binary = "test"
/// arguments = [ "a", "b" ]
///
/// [inputs.test_input]
/// arguments = [ "c" ]
/// ```
///
/// Will run `test a b c`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
pub struct Input {
    /// The path to the input.
    ///
    /// If not specified, nothing is provided on the program's input.
    pub input: Option<PathBuf>,

    /// The additional cli arguments for the executable.
    ///
    /// # Default
    /// By default these will be empty.
    #[serde(default = "EMPTY_ARGS")]
    pub arguments: Vec<String>,
}

/// ### TOML struct that can be used to provide inputs.
/// structure is:
/// ```toml
/// [[inputs]]
/// input = "/path/to/input"
/// arguments = [ "arg1", "arg2" ]
///
/// [[inputs]]
/// input = "/path/to/input2"
/// arguments = [ "arg1", "arg2" ]
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
pub struct InputSchema {
    /// 0 or more `[[input]]` instances
    pub inputs: Vec<Input>,
}

/// A label that can be assigned to a job based on the afterscript output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct Label {
    /// The regex to run over the afterscript output. If there's a match, this
    /// label is assigned.
    pub regex: Regex,

    /// The priority of the label. Higher numbers mean higher priority, and if
    /// label is present it will override lower priority labels, even if
    /// they are also present.
    pub priority: u64,

    /// Whether using rerun failed will rerun this job- ie is this label a
    /// "failure"
    #[serde(default = "RERUN_LABEL_BY_DEFAULT")]
    pub rerun_by_default: bool,
}

/// A config struct used throughout the `gourd` application.
//
// changing the config struct? see notes in ./tests/config.rs
// 1. is the change necessary?
// 2. will it break user workflows?
// 3. update the tests
// 4. update the user documentation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    //
    // Basic settings.
    /// The path to a folder where the experiment output will be stored.
    #[serde(alias = "out")]
    pub output_path: PathBuf,

    /// The path to a folder where the metrics output will be stored.
    #[serde(alias = "metrics")]
    pub metrics_path: PathBuf,

    /// The path to a folder where the experiments will be stored.
    #[serde(alias = "experiments")]
    pub experiments_folder: PathBuf,

    /// The list of tested algorithms.
    #[serde(rename = "program")]
    pub programs: ProgramMap,

    /// The list of inputs for each of them.
    ///
    /// The name of an input cannot contain `glob|`.
    #[serde(rename = "input", default = "INPUTS_DEFAULT")]
    pub inputs: InputMap,

    /// A path to a TOML file that contains input combinations.
    pub input_schema: Option<PathBuf>,

    /// If running on a SLURM cluster, the job configurations
    pub slurm: Option<SlurmConfig>,

    /// If running on a SLURM cluster, the initial global resource limits
    pub resource_limits: Option<ResourceLimits>,

    /// If running on a SLURM cluster, the initial postprocessing resource
    /// limits
    pub postprocess_resource_limits: Option<ResourceLimits>,

    //
    // Advanced settings.
    /// The command to execute to get to the wrapper.
    #[serde(default = "WRAPPER_DEFAULT")]
    pub wrapper: String,

    /// The path to a folder where the afterscript outputs will be stored.
    #[serde(default = "AFTERSCRIPT_OUTPUT_DEFAULT")]
    pub afterscript_output_folder: Option<PathBuf>,

    /// The path to a folder where the afterscript outputs will be stored.
    #[serde(default = "POSTPROCESS_JOB_OUTPUT_DEFAULT")]
    pub postprocess_job_output_folder: Option<PathBuf>,

    /// The list of postprocessing programs.
    #[serde(default = "POSTPROCESS_JOBS_DEFAULT")]
    pub postprocess_programs: Option<ProgramMap>,

    /// Allow custom labels to be assigned based on the afterscript output.
    ///
    /// syntax is:
    /// ```toml
    /// [labels.<label_name>]
    /// regex = "<regex>" # the regex where if it matches then this label is assigned
    /// rerun_by_default = true # whether using rerun failed will rerun this job- ie is this label a "failure"
    /// ```
    #[serde(alias = "label")]
    pub labels: Option<BTreeMap<String, Label>>,
}

/// The config options when running through Slurm
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlurmConfig {
    /// The name of the experiment. This is used (parametrically) as the job
    /// name in SLURM, and for the output directory.
    pub experiment_name: String, /* not sure if we need this user-provided or generated by
                                  * gourd, but it's one of the options and i couldn't tell if
                                  * it's mandatory or not */

    /// Which node partition to use. On DelftBlue, the options are:
    /// - "compute"
    /// - "compute-p2"
    /// - "gpu"
    /// - "gpu-a100"
    /// - "memory"
    /// - "trans"
    /// - "visual"
    pub partition: String, /* technically this would be an enum, but it's different per cluster
                            * so i don't know if we should hardcode delftblue's options */

    /// The maximum number of arrays to schedule at once.
    pub array_count_limit: usize,

    /// The maximum number of jobs to schedule in a Slurm array.
    pub array_size_limit: usize,

    /// Where slurm should put the stdout and stderr of the job.
    pub out: Option<PathBuf>,

    /// Account to charge for this job
    pub account: String,

    /// Delay the run of the job
    pub begin: Option<String>,

    /// Option to set notifications for user by email when a certain event types
    /// occur.
    pub mail_type: Option<String>,

    /// User to be notified by the email (When not specified it's the user that
    /// scheduled the job)
    pub mail_user: Option<String>,

    /// Custom slurm arguments
    pub additional_args: Option<BTreeMap<String, SBatchArg>>,
}

/// The structure for providing custom slurm arguments
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SBatchArg {
    /// Name of the sbatch argument
    pub name: String,

    /// Value of the sbatch argument
    pub value: String,
}

/// The resource limits, a Slurm configuration parameter that can be changed
/// during an experiment. Contains the CPU, time, and memory bounds per run.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum time allowed _for each_ job.
    #[serde(deserialize_with = "duration::deserialize_human_time_duration")]
    pub time_limit: Duration,

    /// CPUs to use per job
    pub cpus: usize,

    /// Memory in MB to allocate per CPU per job
    pub mem_per_cpu: usize,
}

// An implementation that provides a default value of `Config`,
// which allows for the eventual addition of optional config items.
impl Default for Config {
    fn default() -> Self {
        Config {
            output_path: PathBuf::from("run-output"),
            metrics_path: PathBuf::from("run-metrics"),
            experiments_folder: PathBuf::from("experiments"),
            wrapper: WRAPPER_DEFAULT(),
            programs: ProgramMap::default(),
            inputs: InputMap::default(),
            input_schema: None,
            slurm: None,
            resource_limits: None,
            postprocess_resource_limits: None,
            afterscript_output_folder: AFTERSCRIPT_OUTPUT_DEFAULT(),
            postprocess_job_output_folder: POSTPROCESS_JOB_OUTPUT_DEFAULT(),
            postprocess_programs: None,
            labels: Some(BTreeMap::new()),
        }
    }
}

impl Config {
    /// Load a `Config` struct instance from a TOML file at the provided path.
    /// Returns a valid `Config` or an explanatory
    /// `GourdError::ConfigLoadError`.
    pub fn from_file<F: FileOperations>(path: &Path, fs: &F) -> Result<Config> {
        let mut initial: Config = Config::deserialize(UserDeserializer::new(&fs.read_utf8(path)?))
            .with_context(ctx!(
              "Could not parse {path:?}", ;
              "More help and examples can be found with \
              {PRIMARY_STYLE}man gourd.toml{PRIMARY_STYLE:#}",
            ))?;

        if let Some(schema) = &initial.input_schema {
            initial.inputs = Self::parse_schema_inputs(schema.as_path(), initial.inputs, fs)?;
            initial.input_schema = None;
        }

        Ok(initial)
    }

    /// Parse the additional inputs toml file and add them to the inputs map.
    fn parse_schema_inputs(
        path_buf: &Path,
        mut inputs: InputMap,
        fso: &impl FileOperations,
    ) -> Result<InputMap> {
        let hi = fso.try_read_toml::<InputSchema>(path_buf)?;
        for (idx, input) in hi.inputs.iter().enumerate() {
            inputs.insert(
                format!("{}{}{}", INTERNAL_PREFIX, INTERNAL_SCHEMA_INPUTS, idx),
                input.clone(),
            );
        }
        Ok(inputs)
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
