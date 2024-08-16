use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use chrono::DateTime;
use chrono::Local;
use gourd_lib::config::slurm::SlurmConfig;
use gourd_lib::experiment::Experiment;

use crate::chunks::Chunk;
use crate::status::slurm_based::SacctOutput;

/// Some checks when running on slurm to improve error handling
pub mod checks;

/// The core slurm functionality
pub mod handler;
/// Currently used implementation of interacting with SLURM through the CLI
pub mod interactor;

/// The interface for interacting with a SLURM cluster.
/// This can be via a version-specific CLI, via a REST API, or via a library.
pub trait SlurmInteractor {
    /// Check the version of slurm on the current environment.
    /// returns an error if the version is not supported, or if slurm is not
    /// present.
    fn get_version(&self) -> Result<[u64; 2]>;

    /// Check if the provided partition is valid.
    fn get_partitions(&self) -> Result<Vec<Vec<String>>>;

    /// Get the MaxArraySize of this slurm cluster
    fn max_array_size(&self) -> Result<usize>;

    /// Get the MaxSubmit of this slurm cluster
    fn max_submit(&self) -> Result<usize>;

    /// Get the MaxJobs of this slurm cluster
    fn max_jobs(&self) -> Result<usize>;

    /// Get the MaxCPU of this slurm cluster
    fn max_cpu(&self) -> Result<usize>;

    /// Get the MaxMemory of this slurm cluster
    fn max_memory(&self) -> Result<usize>;

    /// Get the MaxWallDurationPerJob of this slurm cluster
    fn max_time(&self) -> Result<Duration>;

    /// Schedule a new job array on the cluster.
    fn schedule_chunk(
        &self,
        slurm_config: &SlurmConfig,
        chunk: &Chunk,
        experiment: &mut Experiment,
        exp_path: &Path,
    ) -> Result<()>;

    /// Check if a version of SLURM is supported by this interactor.
    fn is_version_supported(&self, v: [u64; 2]) -> bool;

    /// Get the supported versions of SLURM for this interactor.
    fn get_supported_versions(&self) -> String;

    /// Get accounting data of user's jobs
    fn get_accounting_data(&self, since: &DateTime<Local>) -> Result<Vec<SacctOutput>>;

    /// Get vector of all (not finished) jobs scheduled by user
    fn scheduled_jobs(&self) -> Result<Vec<String>>;

    /// Get the number of currently scheduled or running jobs by the current
    /// user
    fn scheduled_count(&self) -> Result<usize>;

    /// Cancel all of the jobs in the `batch_ids` vector
    fn cancel_jobs(&self, batch_ids: Vec<String>) -> Result<()>;
}
