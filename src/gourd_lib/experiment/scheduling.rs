use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use anyhow::Context;
use crate::bailc;
use crate::config::ResourceLimits;
use crate::experiment::Experiment;
use crate::experiment::Run;

/// Describes one chunk: a Slurm array of scheduled runs with common resource
/// limits. Chunks are created at runtime; a run is in one chunk iff it has
/// been scheduled.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Chunk {
    /// The runs that belong to this chunk (by RunID)
    pub runs: Vec<usize>,

    /// The resource limits of this chunk.
    ///
    /// This field is immutable.
    resource_limits: ResourceLimits,
}

/// A run status.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum RunStatus {
    /// The job hasn't started yet
    Pending,

    /// The job has started running locally.
    RanLocally,

    /// The run is scheduled on Slurm with a slurm id
    Scheduled(String),

    /// The run has finished (dependent runs can start)
    Finished,
}

impl Chunk {
    /// Get the slurm id of the chunk if it is scheduled.
    ///
    /// Returns None if it is running locally or not ran yet.
    pub fn limits(&self) -> ResourceLimits {
        self.resource_limits.clone()
    }
}

impl Experiment {
    pub fn next_chunks(&mut self, chunk_length: usize) -> Result<Vec<Chunk>> {
        let mut chunks = vec![];

        let runs = self
            .runs
            .iter()
            .enumerate()
            .filter(|(_, r)| r.status == RunStatus::Pending)
            // * when stable, replace with .is_none_or
            // * if you want to implement multiple dependencies (not 1-1) for runs, change here
            .filter(|(_, r)| {
                !r.depends
                    .is_some_and(|d| self.runs[d].status != RunStatus::Finished)
            })
            .collect::<Vec<(usize, &Run)>>();

        if runs.len() == 0 {
            bailc!(
                "No runs left to schedule!",;
                "All available runs have already been scheduled.",;
                "You can rerun some runs, wait for dependent runs to \
                complete, or start a new experiment.",
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

        Ok(chunks)
    }

    pub fn mark_chunk_scheduled(&mut self, chunk: &Chunk, batch_id: String) {
        for run_id in chunk.runs.iter() {
            // TEST:
            // because we schedule an array by specifying the run_id(s) in a list,
            // the sub id should be == run_id.
            // I have not confirmed this though, needs testing
            let job_id = format!("{}_{}", batch_id, run_id);
            self.runs[*run_id].slurm_id = Some(job_id.clone());
        }
    }

    pub fn unscheduled(&self) -> Vec<&Run> {
        self.runs.iter().filter(|r| matches!(r.status, RunStatus::Pending)).collect()
    }
}
