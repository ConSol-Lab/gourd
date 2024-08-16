use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;

/// The config options when running through Slurm
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlurmConfig {
    /// The name of the experiment. This is used (parametrically) as the job
    /// name in SLURM, and for the output directory.
    pub experiment_name: String,

    /// Where slurm should put the stdout and stderr of the job.
    pub output_folder: PathBuf,

    /// Which node partition to use. On DelftBlue, the options are:
    /// - "compute"
    /// - "compute-p2"
    /// - "gpu"
    /// - "gpu-a100"
    /// - "memory"
    /// - "trans"
    /// - "visual"
    pub partition: String,

    /// Override the maximum number of jobs to schedule in a Slurm array.
    ///
    /// If left `None`, a value fetched directly from slurm will be used.
    pub array_size_limit: Option<usize>,

    /// The maximum number of arrays to schedule at once.
    ///
    /// If left `None`, a value fetched directly from slurm will be used.
    pub array_count_limit: Option<usize>,

    /// Account to charge for this job
    pub account: String,

    /// Delay the run of new jobs
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
        deserialize_with = "super::duration::deserialize_human_time_duration",
        serialize_with = "super::duration::serialize_duration"
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
