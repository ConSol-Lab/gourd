use std::collections::BTreeMap;
use std::io::BufWriter;
use std::thread::sleep;

use anyhow::Result;
use gourd_lib::constants::SLURM_VERSIONS;
use gourd_lib::constants::STATUS_REFRESH_PERIOD;
use gourd_lib::experiment::Environment;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use gourd_lib::measurement::Measurement;
use indicatif::MultiProgress;

use self::fs_based::FileBasedProvider;
use self::printing::display_statuses;
use self::slurm_based::SlurmBasedProvider;
use crate::cli::printing::generate_progress_bar;
use crate::slurm::interactor::SlurmCli;

/// File system based status information.
pub mod fs_based;

/// Slurm based status information.
pub mod slurm_based;

/// Printing status information.
pub mod printing;

/// Printing information about scheduled chunks.
pub mod chunks;

/// The reasons for slurm to kill a job
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlurmState {
    /// The job has not yet started.
    Pending,

    /// The job is still running.
    Running,

    /// The job completed successfully.
    Success,

    /// Job terminated due to launch failure, typically due to a hardware
    /// failure (e.g. unable to boot the node or block and the job can not be
    /// requeued).
    BootFail,

    /// Job was explicitly cancelled by the user or system administrator. The
    /// job may or may not have been initiated.
    Cancelled,

    /// Job terminated on deadline.
    Deadline,

    /// Job terminated due to failure of one or more allocated nodes.
    NodeFail,

    /// Job experienced out of memory error.
    OutOfMemory,

    /// Job terminated due to preemption.
    Preempted,

    /// Job has an allocation, but execution has been suspended and CPUs have
    /// been released for other jobs.
    Suspended,

    /// Job reached the time limit
    Timeout,

    /// Unspecified by the account reason to fail
    SlurmFail,
}

impl SlurmState {
    /// Check if this state means that the run is completed.
    pub fn is_completed(&self) -> bool {
        !matches!(self, SlurmState::Pending | SlurmState::Running)
    }
}

/// This possible status of a job, reported by the file system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FsState {
    /// The job has not yet started.
    Pending,

    /// The job is still running.
    Running,

    /// The job completed.
    Completed(Measurement),
}

impl FsState {
    /// Check if this state means that the run is completed.
    pub fn is_completed(&self) -> bool {
        matches!(self, FsState::Completed(_))
    }

    /// Check if this state means that the run has succeded.
    pub fn has_succeeded(&self) -> bool {
        matches!(self, FsState::Completed(Measurement { exit_code: 0, .. }))
    }
}

/// Structure of file based status
#[derive(Debug, Clone, PartialEq)]
pub struct FileSystemBasedStatus {
    /// State of completion of the run
    pub completion: FsState,

    /// If the afterscript completed successfully, this will contain the label,
    /// if one was assigned.
    pub afterscript_completion: Option<Option<String>>,
}

/// Structure of slurm based status
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct SlurmBasedStatus {
    /// State of completion of the run
    pub completion: SlurmState,

    /// Exit code of the program
    pub exit_code_program: isize,

    /// Exit code of the slurm
    pub exit_code_slurm: isize,
}

/// All possible postprocessing statuses of a run.
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    /// Status retrieved from slurm.
    pub slurm_status: Option<SlurmBasedStatus>,

    /// Status retrieved from the filesystem.
    pub fs_status: FileSystemBasedStatus,
}

impl Status {
    /// Check if we know this job to be completed based on the status.
    ///
    /// Completed also includes failed jobs.
    pub fn is_completed(&self) -> bool {
        if self.fs_status.completion.is_completed() {
            true
        } else {
            self.slurm_status
                .map(|x| x.completion.is_completed())
                .unwrap_or(false)
        }
    }

    /// Check if we know this job to have failed.
    pub fn has_failed(&self, experiment: &Experiment) -> bool {
        let a = match self.fs_status.completion {
            FsState::Completed(Measurement { exit_code, .. }) => exit_code != 0,
            _ => false,
        };
        let b = match self.slurm_status {
            Some(SlurmBasedStatus {
                completion: SlurmState::BootFail,
                ..
            }) => true,
            Some(SlurmBasedStatus {
                completion: SlurmState::Cancelled,
                ..
            }) => true,
            Some(SlurmBasedStatus {
                completion: SlurmState::Deadline,
                ..
            }) => true,
            Some(SlurmBasedStatus {
                completion: SlurmState::NodeFail,
                ..
            }) => true,
            Some(SlurmBasedStatus {
                completion: SlurmState::OutOfMemory,
                ..
            }) => true,
            Some(SlurmBasedStatus {
                completion: SlurmState::Preempted,
                ..
            }) => true,
            Some(SlurmBasedStatus {
                completion: SlurmState::Timeout,
                ..
            }) => true,
            Some(SlurmBasedStatus {
                completion: SlurmState::SlurmFail,
                ..
            }) => true,
            Some(_) => false,
            None => false,
        };
        let c = match &self.fs_status.afterscript_completion {
            Some(Some(label)) => {
                if let Some(l) = &experiment.labels.map.get(label) {
                    l.rerun_by_default
                } else {
                    false
                }
            }
            _ => false,
        };
        a || b || c
    }

    /// Check if we know this job to be scheduled.
    pub fn is_scheduled(&self) -> bool {
        self.slurm_status.is_some()
    }
}

/// This type maps between `run_id` and the [Status] of the run.
pub type ExperimentStatus = BTreeMap<usize, Status>;

/// A struct that can attest the statuses or some or all running jobs.
pub trait StatusProvider<T, ST> {
    /// Try to get the statuses of jobs.
    fn get_statuses(connection: &mut T, experiment: &Experiment) -> Result<BTreeMap<usize, ST>>;
}

/// Get the status of the provided experiment.
pub fn get_statuses(
    experiment: &Experiment,
    fs: &mut impl FileOperations,
) -> Result<ExperimentStatus> {
    let fs_status = FileBasedProvider::get_statuses(fs, experiment)?;

    let mut slurm = SlurmCli {
        versions: SLURM_VERSIONS.to_vec(),
    };

    let slurm_status = if experiment.env == Environment::Slurm {
        Some(SlurmBasedProvider::get_statuses(&mut slurm, experiment)?)
    } else {
        None
    };

    merge_statuses(fs_status, slurm_status, 0..experiment.runs.len())
}

/// A function that merges status providers outputs.
pub fn merge_statuses(
    fs: BTreeMap<usize, FileSystemBasedStatus>,
    slurm: Option<BTreeMap<usize, SlurmBasedStatus>>,
    jobs: impl Iterator<Item = usize>,
) -> Result<ExperimentStatus> {
    let mut out = BTreeMap::<usize, Status>::new();

    for job_id in jobs {
        if let Some(slurm_based) = slurm.as_ref() {
            out.insert(
                job_id,
                Status {
                    slurm_status: slurm_based.get(&job_id).cloned(),
                    fs_status: fs[&job_id].clone(),
                },
            );
        } else {
            out.insert(
                job_id,
                Status {
                    slurm_status: None,
                    fs_status: fs[&job_id].clone(),
                },
            );
        }
    }

    Ok(out)
}

/// Print status until all tasks are finished.
pub fn blocking_status(
    progress: &MultiProgress,
    experiment: &Experiment,
    fs: &mut impl FileOperations,
    full: bool,
) -> Result<()> {
    let mut complete = 0;
    let mut message = "".to_string();

    let bar = progress.add(generate_progress_bar(experiment.runs.len() as u64)?);

    while complete < experiment.runs.len() {
        let mut buf = BufWriter::new(Vec::new());

        let statuses = get_statuses(experiment, fs)?;

        complete = display_statuses(&mut buf, experiment, &statuses, full)?;
        message = format!("{}\n", String::from_utf8(buf.into_inner()?)?);

        bar.set_prefix(message.clone());
        bar.set_position(complete as u64);

        sleep(STATUS_REFRESH_PERIOD);
    }

    bar.finish();
    progress.remove(&bar);
    progress.clear()?;

    let leftover = generate_progress_bar(experiment.runs.len() as u64)?;
    leftover.set_position(complete as u64);
    leftover.set_prefix(message);
    leftover.finish();

    Ok(())
}
