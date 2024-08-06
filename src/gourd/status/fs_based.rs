use std::collections::BTreeMap;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use gourd_lib::ctx;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use gourd_lib::measurement::Metrics;
use log::debug;
use log::trace;
use log::warn;

use super::FileSystemBasedStatus;
use super::StatusProvider;
use crate::post::afterscript::run_afterscript;
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
                "Reading status for run {} from {:?}",
                run_id,
                run.metrics_path
            );

            let metrics = match fs.try_read_toml::<Metrics>(&run.metrics_path) {
                Ok(x) => Some(x),
                Err(e) => {
                    trace!("Failed to read metrics: {:?}", e);
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

            if run.afterscript_output_path.is_some() && completion.has_succeeded() {
                afterscript_completion = match Self::get_afterscript_status(run_id, experiment, fs)
                {
                    Ok(status) => Some(status),
                    Err(e) => {
                        warn!("Failed to get status from afterscript {}.", run_id);
                        debug!("{}", e);
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
    pub fn get_afterscript_status(
        run_id: usize,
        exp: &Experiment,
        fs: &impl FileOperations,
    ) -> Result<Option<String>> {
        let run = &exp.runs[run_id];

        let file = run
            .afterscript_output_path
            .clone()
            .ok_or(anyhow!("Could not get the afterscript information"))
            .with_context(ctx!(
                "Could not get the postprocessing information", ;
                "",
            ))?;

        if !file.exists() {
            run_afterscript(run_id, exp)?;
        }

        assign_label(exp, &file, fs)
    }
}
