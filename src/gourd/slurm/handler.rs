use std::cmp::min;
use std::ops::Div;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::bail;
use anyhow::ensure;
use anyhow::Result;
use gourd_lib::config::slurm::ResourceLimits;
use gourd_lib::config::slurm::SlurmConfig;
use gourd_lib::constants::MAIL_TYPE_VALID_OPTIONS;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use log::debug;
use log::error;
use log::warn;

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

        let max_submit = if let Some(custom) = slurm_config.array_count_limit {
            custom
        } else {
            self.internal.max_submit()?
        };
        debug!("Max Submit: {max_submit}");

        // capacity to still schedule runs, taking into account everything
        // the current user is running (not just this experiment)
        let capacity = max_submit - self.internal.scheduled_count()?;
        debug!("Capacity: {capacity}");
        ensure!(
            capacity > 0,
            "Your Slurm queue is full! Scheduling any more runs will be blocked. \
            Please wait for your other runs to finish, or cancel them before scheduling more"
        );

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

    /// Verify that a set of resource limits adhere to the cluster rules
    pub fn verify_resource_limits(&self, resource_limits: &ResourceLimits) -> Result<()> {
        match resource_limits.cpus {
            0 => bail!("CPUs must be greater than 0"),
            x => {
                let max = self.internal.max_cpu()?;
                if x > max {
                    bail!(
                        "{} cpus requested per task, but only {} allowed by the cluster",
                        x,
                        max
                    )
                }
            }
        };

        if resource_limits.mem_per_cpu == 0 {
            warn!("Memory limit set to 0, this grants access to all the memory available on the node.");
        }

        match resource_limits.time_limit {
            x if x <= Duration::default() => bail!("Time must be greater than 0"),
            x => {
                let max = self.internal.max_time()?;
                if x > max {
                    bail!(
                        "{} time requested per task, but only {} allowed by the cluster",
                        humantime::format_duration(x),
                        humantime::format_duration(max),
                    )
                }
            }
        }

        Ok(())
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
