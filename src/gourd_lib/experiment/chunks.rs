use serde::{Deserialize, Serialize};

/// Describes one chunk: a Slurm array of scheduled runs with common resource
/// limits. Chunks are created at runtime; a run is in one chunk iff it has been
/// scheduled.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Chunk {
    /// The runs that belong to this chunk (by RunID)
    pub runs: Vec<usize>,

    // just here for reference
    // /// The resource limits of this chunk.
    // pub resource_limits: Option<ResourceLimits>,

    /// Whether this chunk has been run or not.
    pub status: ChunkRunStatus,
}

/// The run status of a chunk.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ChunkRunStatus {
    /// The job hasn't started yet
    Pending,

    /// The job has started running locally.
    RanLocally,

    /// The run is scheduled on Slurm with a slurm id
    Scheduled(String),
}

impl Chunk {
    /// Get the slurm id of the chunk if it is scheduled.
    ///
    /// Returns None if it is running locally or not ran yet.
    pub fn get_slurm_id(&self) -> Option<String> {
        match self.status {
            ChunkRunStatus::Scheduled(ref id) => Some(id.clone()),
            _ => None,
        }
    }
}
