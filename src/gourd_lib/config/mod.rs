use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

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

/// References to Git submodules.
pub mod git;

/// Slurm configuration.
pub mod slurm;

pub use regex::Regex;

use crate::config::slurm::ResourceLimits;
use crate::config::slurm::SlurmConfig;

/// A pair of a path to a binary and cli arguments.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
#[serde(deny_unknown_fields)]
pub struct UserProgram {
    /// A path to the executable.
    pub binary: Option<PathBuf>,

    /// Fetch the program binary remotely.
    /// ### Permissions
    /// If this file is fetched on unix, the permissions
    /// for it are: `rwxr-xr--`.
    pub fetch: Option<FetchedResource<0o754>>,

    /// Fetch the program as a file from a Git repository.
    pub git: Option<GitResource>,

    /// The cli arguments for the executable.
    #[serde(default = "EMPTY_ARGS")]
    pub arguments: Vec<String>,

    /// Environment variables for the executable (key, value).
    pub env: Option<BTreeMap<String, String>>,

    /// The path to the afterscript, if there is one.
    #[serde(default = "AFTERSCRIPT_DEFAULT")]
    pub afterscript: Option<PathBuf>,

    /// Resource limits to optionally overwrite default resource limits.
    #[serde(default)]
    pub resource_limits: Option<ResourceLimits>,

    /// The programs to postprocess this one.
    #[serde(default)]
    pub next: Vec<String>,
}

/// Fetch a remote resource
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
pub struct FetchedResource<const PERMISSIONS: u32> {
    /// The url from which to fetch this resource.
    pub url: String,
    /// The file in which to store this resource.
    pub store: PathBuf,
}

const fn _default_true() -> bool {
    true
}

/// Fetch a resource from a Git submodule.
///
/// Optionally, at most one of the `commit` and `tag` fields can be used to
/// override the default behavior of using the repository's main branch.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
pub struct GitResource {
    /// The name of the submodule to fetch from.
    pub submodule_name: String,

    #[serde(default)]
    /// The commit/tag to checkout within the submodule.
    pub rev: Option<String>,

    #[serde(default)]
    /// The command to run prior to retrieving the file.
    pub compile_script: Option<String>,

    /// The file to copy from the Git repository.
    pub file: PathBuf,

    /// The location to store the copied file in.
    pub store: PathBuf,

    #[serde(default = "_default_true")]
    /// Whether to 'git fetch' if a commit cannot be found.
    pub enable_fetch: bool,
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

    /// Fetch the program as a file from a Git repository.
    pub git: Option<GitResource>,

    /// Fetch the input file remotely
    /// ### Permissions
    /// If this file is fetched on unix, the permissions
    /// for it are: `rw-r--r--`.
    pub fetch: Option<FetchedResource<0o644>>,

    /// Mark this input as belonging to a specific group of inputs.
    pub group: Option<String>,

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

    /// The list of resources to fetch from Git, for use in parameter literals.
    #[serde(rename = "git_resource")]
    pub git_resources: Option<BTreeMap<String, GitResource>>,

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

// An implementation that provides a default value of `Config`,
// which allows for the eventual addition of optional config items.
impl Default for Config {
    fn default() -> Self {
        Config {
            output_path: PathBuf::from("run-output"),
            metrics_path: PathBuf::from("run-metrics"),
            experiments_folder: PathBuf::from("experiments"),
            git_resources: None,
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

        // Obtain resources copied from a Git repository
        // (this may contain the schema, so must happen before it)
        match &initial.git_resources {
            Some(git_resource_map) => {
                for resource in git_resource_map {
                    resource.1.fetch(fs)?;
                }
            },
            None => {},
        }   

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
