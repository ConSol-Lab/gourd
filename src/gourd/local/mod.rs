use std::env;

use anyhow::Result;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use log::trace;

use self::runner::run_locally;
use crate::status::DynamicStatus;
use crate::wrapper::wrap;

/// The (first iteration) thread pool implementation.
pub mod runner;

/// Run an experiment locally, as specified in the config file.
pub async fn run_local(
    experiment: &mut Experiment,
    fs: &impl FileOperations,
    force: bool,
    sequential: bool,
) -> Result<usize> {
    let status = experiment.status(fs)?;
    let pre_fin = status.iter().filter(|r| r.1.is_completed()).count();

    let cmds = wrap(experiment, &status, env::consts::ARCH, fs)?;

    trace!("About to run: {cmds:#?}");

    experiment.save(fs)?;

    let len = cmds.len();
    run_locally(cmds, force, sequential).await?;

    Ok(len + pre_fin)
}
