use std::path::PathBuf;

use anyhow::Result;
use gourd_lib::config::SlurmConfig;
use gourd_lib::constants::MAIL_TYPE_VALID_OPTIONS;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use log::debug;
use log::error;

use crate::slurm::checks::slurm_options_from_experiment;
use crate::slurm::SlurmInteractor;

/// Functionality associated with running on slurm
#[derive(Debug, Clone, Copy)]
pub struct SlurmHandler<T>
where
    T: SlurmInteractor,
{
    /// The way of interaction with slurm. (May be cli or library based).
    pub internal: T,
}

impl<T> Default for SlurmHandler<T>
where
    T: SlurmInteractor + Default,
{
    fn default() -> Self {
        Self {
            internal: T::default(),
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl<T> SlurmHandler<T>
where
    T: SlurmInteractor,
{
    /// Run an experiment on delftblue.
    ///
    /// # Returns
    /// The amount of chunks that have been scheduled.
    pub fn run_experiment(
        &self,
        experiment: &mut Experiment,
        exp_path: PathBuf,
        fs: impl FileOperations,
    ) -> Result<usize> {
        let slurm_config = slurm_options_from_experiment(experiment)?;

        let chunks_to_schedule = experiment.next_chunks(slurm_config.array_size_limit)?;

        let mut counter = 0;
        for (chunk_id, chunk) in chunks_to_schedule.iter().enumerate() {
            debug!(
                "Scheduling chunk {} with {} runs",
                chunk_id,
                chunk.runs.len()
            );

            if let Err(e) = self.internal.schedule_chunk(
                &slurm_config,
                chunk,
                experiment,
                &fs.canonicalize(&exp_path)?,
            ) {
                error!("Could not schedule chunk #{}: {:?}", chunk_id, e);
                break;
            }

            counter += 1;
        }
        experiment.save(&fs)?;

        Ok(counter)
    }
}

/// Helper function to create string with optional args for slurm
pub fn parse_optional_args(slurm_config: &SlurmConfig) -> String {
    let mut result = "".to_string();

    if let Some(val) = &slurm_config.begin {
        result.push_str(&format!("#SBATCH --begin={}\n", val));
    }

    if let Some(val) = &slurm_config.mail_type {
        assert!(MAIL_TYPE_VALID_OPTIONS.contains(&val.as_str()));
        result.push_str(&format!("#SBATCH --mail-type={}\n", val))
    }

    if let Some(val) = &slurm_config.mail_user {
        result.push_str(&format!("#SBATCH --mail-user={}\n", val))
    }

    if let Some(args) = &slurm_config.additional_args {
        for arg in args.values() {
            result.push_str(&format!("#SBATCH --{}={}\n", arg.name, arg.value))
        }
    }

    result
}

#[cfg(test)]
#[path = "tests/handler.rs"]
mod tests;
