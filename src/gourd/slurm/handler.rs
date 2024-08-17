use std::cmp::min;
use std::ops::Div;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::config::slurm::SlurmConfig;
use gourd_lib::constants::CMD_DOC_STYLE;
use gourd_lib::constants::MAIL_TYPE_VALID_OPTIONS;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use log::debug;
use log::error;

use crate::chunks::Chunkable;
use crate::slurm::checks::slurm_options_from_experiment;
use crate::slurm::SlurmInteractor;
use crate::status::DynamicStatus;

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
    /// ### Returns
    /// The amount of new chunks that have been scheduled.
    pub fn run_experiment(
        &self,
        experiment: &mut Experiment,
        exp_path: PathBuf,
        fs: &impl FileOperations,
    ) -> Result<usize> {
        let slurm_config = slurm_options_from_experiment(experiment)?;

        let status = experiment.status(fs)?;
        let max_array_size = if let Some(custom) = slurm_config.array_size_limit {
            custom
        } else {
            self.internal.max_array_size()?
        };
        debug!("Max Array Size: {max_array_size}");

        let max_submit = if let Some(custom) = slurm_config.max_submit {
            custom
        } else {
            self.internal.max_submit()?
        };
        debug!("Max Submit: {max_submit}");

        // capacity to still schedule runs, taking into account everything
        // the current user is running (not just this experiment)
        let sched = self.internal.scheduled_count()?;
        let capacity = max_submit - sched;
        debug!("Capacity: {capacity}");
        if capacity == 0 {
            bailc!(
                "You cannot schedule any more runs currently, scheduled: {sched}, max_submit: {}",
                max_submit;
                "Your Slurm queue is full! Scheduling more runs will be blocked by Slurm.",;
                "Please wait for your other runs to finish, or cancel them to scheduling more.\n\
                If this is incorrect, you can manually specify \
                {CMD_DOC_STYLE}max_submit{CMD_DOC_STYLE:#} in the slurm configuration to bypass.",
            );
        }

        let max_chunk_size = min(max_array_size, capacity);
        debug!("Max Chunk Size: {max_chunk_size}");

        let max_next_chunks = capacity.div(min(max_array_size, capacity));
        debug!("Max Next Chunks: {max_next_chunks}");

        let chunks_to_schedule = experiment.next_chunks(max_chunk_size, max_next_chunks, status)?;

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
        experiment.save(fs)?;

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
