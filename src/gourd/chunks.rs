use std::cmp::Ordering;

use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::config::ResourceLimits;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::Run;
use serde::Deserialize;
use serde::Serialize;

use crate::status::ExperimentStatus;

/// Describes one chunk: a Slurm array of scheduled runs with common resource
/// limits. Chunks are created at runtime; a run is in one chunk iff it has
/// been scheduled.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// The runs that belong to this chunk (by RunID)
    pub runs: Vec<usize>,

    /// The resource limits of this chunk.
    ///
    /// This field is immutable.
    resource_limits: ResourceLimits,
}

impl Chunk {
    /// Get the slurm id of the chunk if it is scheduled.
    ///
    /// Returns None if it is running locally or not ran yet.
    pub fn limits(&self) -> ResourceLimits {
        self.resource_limits
    }
}

impl PartialOrd for Chunk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Chunk {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.runs.len().cmp(&other.runs.len()) != Ordering::Equal {
            self.runs.len().cmp(&other.runs.len())
        } else {
            self.resource_limits.cmp(&other.resource_limits)
        }
    }
}

/// Split an [`Experiment`]'s [`Run`]s into [`Chunk`]s of common
/// [`ResourceLimits`].
pub trait Chunkable {
    /// Next available [`Chunk`]s for scheduling,
    fn next_chunks(&mut self, chunk_length: usize, status: ExperimentStatus) -> Result<Vec<Chunk>>;
    /// Once a chunk has been scheduled, mark all of its runs as scheduled with
    /// their slurm ids
    fn mark_chunk_scheduled(&mut self, chunk: &Chunk, batch_id: String);
    /// Get the still pending runs of this experiment.
    fn unscheduled(&self, status: &ExperimentStatus) -> Vec<(usize, &Run)>;
    /// Get the still pending runs of this experiment but mutable.
    fn unscheduled_mut(&mut self, status: &ExperimentStatus) -> Vec<(usize, &mut Run)>;
}

impl Chunkable for Experiment {
    fn next_chunks(&mut self, chunk_length: usize, status: ExperimentStatus) -> Result<Vec<Chunk>> {
        let mut chunks = vec![];

        let runs: Vec<(usize, &Run)> = self.unscheduled(&status);

        if runs.is_empty() {
            bailc!(
                "No runs left to schedule!",;
                "All available runs have already been scheduled.",;
                "You can rerun, wait for dependent runs to complete, or start a new experiment.",
            );
        }

        let separated = runs
            .chunk_by(|a, b| a.1.limits == b.1.limits)
            .collect::<Vec<&[(usize, &Run)]>>();

        for c in separated {
            for f in c.chunks(chunk_length) {
                chunks.push(Chunk {
                    runs: f.iter().map(|(i, _)| *i).collect(),
                    resource_limits: f[0].1.limits,
                });
            }
        }

        chunks.sort_unstable();
        chunks.reverse();
        // decreasing order of size, such that we schedule as much as possible first

        Ok(chunks)
    }

    fn mark_chunk_scheduled(&mut self, chunk: &Chunk, batch_id: String) {
        for run_id in chunk.runs.iter() {
            // because we schedule an array by specifying the run_id(s) in a list,
            // the sub id should be == run_id.
            self.runs[*run_id].slurm_id = Some(format!("{}_{}", batch_id, run_id));
        }
    }

    fn unscheduled(&self, status: &ExperimentStatus) -> Vec<(usize, &Run)> {
        self.runs
            .iter()
            .enumerate()
            .filter(|(r_idx, r)| {
                !status[r_idx].is_scheduled()
                    && !status[r_idx].is_completed()
                    && r.slurm_id.is_none()
            })
            .filter(|(_, r)| !r.parent.is_some_and(|d| !status[&d].is_completed()))
            .collect()
    }

    fn unscheduled_mut(&mut self, status: &ExperimentStatus) -> Vec<(usize, &mut Run)> {
        self.runs
            .iter_mut()
            .enumerate()
            .filter(|(r_idx, r)| {
                !status[r_idx].is_scheduled()
                    && !status[r_idx].is_completed()
                    && r.slurm_id.is_none()
            })
            .filter(|(_, r)| !r.parent.is_some_and(|d| !status[&d].is_completed()))
            .collect()
    }
}
