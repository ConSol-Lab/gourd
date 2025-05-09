use std::collections::BTreeMap;

use anyhow::Result;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use gourd_lib::measurement::Metrics;
use log::trace;
use log::warn;

use super::FileSystemBasedStatus;
use super::StatusProvider;
use crate::post::labels::assign_label;
use crate::status::FsState;

/// Provide job status information based on the files system information.
#[derive(Debug, Clone, Copy)]
pub struct FileBasedProvider {}

impl<T> StatusProvider<T, FileSystemBasedStatus> for FileBasedProvider
where
    T: FileOperations,
{
    fn get_statuses(
        fs: &T,
        experiment: &Experiment,
    ) -> Result<BTreeMap<usize, FileSystemBasedStatus>> {
        let mut statuses = BTreeMap::new();

        for (run_id, run) in experiment.runs.iter().enumerate() {
            trace!(
                "Reading status for run {run_id} from {:?}",
                run.metrics_path
            );

            let metrics = match fs.try_read_toml::<Metrics>(&run.metrics_path) {
                Ok(x) => Some(x),
                Err(e) => {
                    trace!("Failed to read metrics: {e:?}");

                    None
                }
            };

            let completion = match metrics {
                Some(inner) => match inner {
                    Metrics::Done(metrics) => FsState::Completed(metrics),
                    Metrics::NotCompleted => FsState::Running,
                },
                None => FsState::Pending,
            };

            let mut afterscript_completion = None;

            if run.afterscript_output.is_some() && completion.has_succeeded() {
                afterscript_completion = match Self::get_afterscript_status(run_id, experiment) {
                    Ok(status) => Some(status),
                    Err(e) => {
                        warn!(
                            "No output found for afterscript of run #{run_id}: {e}\n\
                            Check that the afterscript correctly places the output in a file.",
                        );
                        None
                    }
                };
            }

            let status = FileSystemBasedStatus {
                completion,
                afterscript_completion,
            };

            statuses.insert(run_id, status);
        }

        Ok(statuses)
    }
}

impl FileBasedProvider {
    /// Get the completion of an afterscript.
    pub fn get_afterscript_status(run_id: usize, exp: &Experiment) -> Result<Option<String>> {
        let run = &exp.runs[run_id];

        if let Some(text_output) = run.afterscript_output.clone() {
            assign_label(run_id, &text_output, exp)
        } else {
            Ok(None)
        }
    }
}
