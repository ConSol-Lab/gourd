use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::AFTERSCRIPT_DEFAULT;
use crate::constants::CMD_STYLE;
use crate::constants::EMPTY_ARGS;
use crate::constants::INTERNAL_PREFIX;
use crate::constants::INTERNAL_SCHEMA_INPUTS;
use crate::constants::LABEL_OVERLAP_DEFAULT;
use crate::constants::PROGRAM_RESOURCES_DEFAULT;
use crate::constants::RERUN_LABEL_BY_DEFAULT;
use crate::constants::WRAPPER_DEFAULT;
use crate::error::ctx;
use crate::file_system::FileOperations;
use crate::file_system::FileSystemInteractor;

/// Deserializer for the duration.
mod duration;

/// Deserializer for the maps.
pub mod maps;

/// Deserializer for the regexes.
mod regex;

/// Deserializer for the paramters (grid search).
pub mod parameters;

/// Fetching for resources.
pub mod fetching;

pub use regex::Regex;

/// A pair of a path to a binary and cli arguments.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
#[serde(deny_unknown_fields)]
pub struct UserProgram {
    /// A path to the executable.
    pub binary: Option<PathBuf>,

    /// Fetch the program binary remotely
    /// ### Permissions
    /// If this file is fetched on unix, the permissions
    /// for it are: `rwxr-xr--`.
    pub fetch: Option<FetchedResource<0o754>>,

    /// The cli arguments for the executable.
    #[serde(default = "EMPTY_ARGS")]
    pub arguments: Vec<String>,

    /// The path to the afterscript, if there is one.
    #[serde(default = "AFTERSCRIPT_DEFAULT")]
    pub afterscript: Option<PathBuf>,

    /// Resource limits to optionally overwrite default resource limits.
    #[serde(default = "PROGRAM_RESOURCES_DEFAULT")]
    pub resource_limits: Option<ResourceLimits>,

    /// The programs to postprocess this one.
    #[serde(default)]
    pub next: Vec<String>,
}

/// Fetch a remote resource
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
pub struct FetchedResource<const PERMISSIONS: u32> {
    /// The url from which to fetch this resource
    pub url: String,
    /// The file in which to store this resource
    pub store: PathBuf,
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
#[serde(deny_unknown_fields)]
pub struct UserInput {
    /// Direct path to the input.
    pub file: Option<PathBuf>,

    /// A glob of input files
    pub glob: Option<String>,

    /// Fetch the input file remotely
    /// ### Permissions
    /// If this file is fetched on unix, the permissions
    /// for it are: `rw-r--r--`.
    pub fetch: Option<FetchedResource<0o644>>,

    /// The additional cli arguments for the executable.
    ///
    /// ### Default
    /// By default these will be empty.
    #[serde(default = "EMPTY_ARGS")]
    pub arguments: Vec<String>,
}

/// ### TOML struct that can be used to provide inputs.
/// structure is:
/// ```toml
/// [[input]]
/// input = "/path/to/input"
/// arguments = [ "arg1", "arg2" ]
///
/// [[input]]
/// input = "/path/to/input2"
/// arguments = [ "arg1", "arg2" ]
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
#[serde(deny_unknown_fields)]
pub struct InputSchema {
    /// 0 or more `[[input]]` instances
    #[serde(rename = "input")]
    pub inputs: Vec<UserInput>,
}

/// A parameter.
///
/// # Examples
///
/// ```toml
/// [parameters.x]
/// values = ["1", "2"]
///
/// [parameters.y]
/// values = ["a", "b"]
///
/// [programs.test_program]
/// binary = "test"
///
/// [inputs.test_input]
/// arguments = [ "param|x" ]
/// ```
///
/// Will run:
/// `test 1 a`
/// `test 1 b`
/// `test 2 a`
/// `test 2 b`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    /// Sub-parameters of this parameter.
    ///
    /// To be used exclusively without values of parameter.
    pub sub: Option<BTreeMap<String, SubParameter>>,

    /// The values of parameter.
    ///
    /// To be used exclusively without sub (parameter).
    pub values: Option<Vec<String>>,
}

/// A subparameter.
///
/// # Examples
///
/// ```toml
/// [parameters.x.sub.a]
/// values = ["1", "2", "3"]
///
/// [parameters.x.sub.b]
/// values = ["15", "60", "30"]
///
/// [programs.test_program]
/// binary = "test"
///
/// [inputs.test_input]
/// arguments = [ "subparam|x.a", "subparam|x.b" ]
/// ```
///
/// Will run:
/// `test 1 15`
/// `test 2 60`
/// `test 3 30`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
#[serde(deny_unknown_fields)]
pub struct SubParameter {
    /// The values of sub parameter.
    ///
    /// Has to be equal in length to values of other subparameters of the same
    /// argument.
    pub values: Vec<String>,
}

/// A label that can be assigned to a job based on the afterscript output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
#[serde(deny_unknown_fields)]
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
    pub output_path: PathBuf,

    /// The path to a folder where the metrics output will be stored.
    pub metrics_path: PathBuf,

    /// The path to a folder where the experiments will be stored.
    pub experiments_folder: PathBuf,

    /// The list of tested algorithms.
    #[serde(rename = "program")]
    pub programs: BTreeMap<String, UserProgram>,

    /// The list of inputs for each of them.
    ///
    /// The name of an input cannot contain '_i_'.
    #[serde(rename = "input")]
    pub inputs: BTreeMap<String, UserInput>,

    /// A path to a TOML file that contains input combinations.
    pub input_schema: Option<PathBuf>,

    /// The list of parameters.
    #[serde(rename = "parameter")]
    pub parameters: Option<BTreeMap<String, Parameter>>,

    /// If running on a SLURM cluster, the job configurations.
    pub slurm: Option<SlurmConfig>,

    /// If running on a SLURM cluster, the initial global resource limits.
    pub resource_limits: Option<ResourceLimits>,

    //
    // Advanced settings.
    /// The command to execute to get to the wrapper.
    #[serde(default = "WRAPPER_DEFAULT")]
    pub wrapper: String,

    /// Allow custom labels to be assigned based on the afterscript output.
    ///
    /// syntax is:
    /// ```toml
    /// [labels.<label_name>]
    /// // the regex where if it matches then this label is assigned
    /// regex = "<regex>"
    /// // whether using rerun failed will rerun this job-
    /// // i.e. is this label a "failure"
    /// rerun_by_default = true
    /// ```
    #[serde(rename = "label")]
    pub labels: Option<BTreeMap<String, Label>>,

    /// If set to true, will throw an error when multiple labels are present in
    /// afterscript output.
    #[serde(default = "LABEL_OVERLAP_DEFAULT")]
    pub warn_on_label_overlap: bool,
}

/// The config options when running through Slurm
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct SBatchArg {
    /// Name of the sbatch argument
    pub name: String,

    /// Value of the sbatch argument
    pub value: String,
}

/// The resource limits, a Slurm configuration parameter that can be changed
/// during an experiment. Contains the CPU, time, and memory bounds per run.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    /// Maximum time allowed _for each_ job.
    #[serde(
        deserialize_with = "duration::deserialize_human_time_duration",
        serialize_with = "duration::serialize_duration"
    )]
    pub time_limit: Duration,

    /// CPUs to use per job
    pub cpus: usize,

    /// Memory in MB to allocate per CPU per job
    pub mem_per_cpu: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        ResourceLimits {
            time_limit: Duration::from_secs(60),
            cpus: 1,
            mem_per_cpu: 32,
        }
    }
}

impl Display for ResourceLimits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "  time limit: {}",
            humantime::format_duration(self.time_limit)
        )?;
        writeln!(f, "  cpus: {}", self.cpus)?;
        writeln!(f, "  memory per cpu: {}MB", self.mem_per_cpu)
    }
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
            programs: BTreeMap::default(),
            inputs: BTreeMap::default(),
            input_schema: None,
            parameters: None,
            slurm: None,
            resource_limits: None,
            labels: Some(BTreeMap::new()),
            warn_on_label_overlap: true,
        }
    }
}

impl Config {
    /// Load a `Config` struct instance from a TOML file at the provided path.
    /// Returns a valid `Config` or an explanatory
    /// `GourdError::ConfigLoadError`.
    pub fn from_file(path: &Path, fs: &FileSystemInteractor) -> Result<Config> {
        let mut initial: Config = fs.try_read_toml(path).with_context(ctx!(
          "Could not parse {path:?}", ;
          "More help and examples can be found with \
          {CMD_STYLE}man gourd.toml{CMD_STYLE:#}",
        ))?;

        if let Some(schema) = &initial.input_schema {
            initial.inputs = Config::parse_schema_inputs(schema.as_path(), initial.inputs, fs)?;
            initial.input_schema = None;
        }

        Ok(initial)
    }

    /// Parse the additional inputs toml file and add them to the inputs map.
    pub fn parse_schema_inputs(
        path_buf: &Path,
        mut inputs: BTreeMap<String, UserInput>,
        fso: &impl FileOperations,
    ) -> Result<BTreeMap<String, UserInput>> {
        let hi = fso.try_read_toml::<InputSchema>(path_buf)?;

        for (idx, input) in hi.inputs.iter().enumerate() {
            inputs.insert(
                format!("{}{}{}", idx, INTERNAL_PREFIX, INTERNAL_SCHEMA_INPUTS),
                input.clone(),
            );
        }

        Ok(inputs)
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
