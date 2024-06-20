/// User friendliness
pub mod checks;
/// Get the runs
pub mod runs;
/// Handle statuses
pub mod status;

use std::fmt::Display;
use std::fmt::Formatter;

use gourd_lib::config::ResourceLimits;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::FieldRef;

/// The status of a single run
#[derive(Ord, PartialOrd, Eq, PartialEq, Clone)]
pub(super) enum RerunStatus {
    /// Run has not finished yet
    NotFinished,

    /// Finished successfully, with exit code 0
    FinishedExitZero,

    /// Finished successfully, and the assigned label has rerun_by_default set
    /// to false
    FinishedSuccessLabel(String),

    /// Failed because the assigned label has rerun_by_default set to true
    FailedErrorLabel(String),

    /// Failed with an exit code
    FailedExitCode(i32),
}

impl Display for RerunStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RerunStatus::NotFinished => write!(f, "Not finished"),
            RerunStatus::FinishedExitZero => write!(f, "Finished with exit code 0"),
            RerunStatus::FinishedSuccessLabel(l) => write!(f, "Finished with label {}", l),
            RerunStatus::FailedErrorLabel(l) => write!(f, "Failed with label {}", l),
            RerunStatus::FailedExitCode(c) => write!(f, "Failed with exit code {}", c),
        }
    }
}

/// Find and update the resource limits for the program of a run.
fn update_program_resource_limits(
    run_id: usize,
    experiment: &mut Experiment,
    new_rss: ResourceLimits,
) {
    match &experiment.runs[run_id].program {
        FieldRef::Regular(name) => {
            experiment
                .config
                .programs
                .get_mut(name)
                .unwrap()
                .resource_limits = Some(new_rss);
        }
        FieldRef::Postprocess(name) => {
            experiment
                .config
                .postprocess_programs
                .as_mut()
                .unwrap()
                .get_mut(name)
                .unwrap()
                .resource_limits = Some(new_rss);
        }
    }
}