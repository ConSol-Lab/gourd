use std::env;
use std::path::Path;

use anyhow::Result;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use log::trace;

use self::runner::run_locally;
use crate::wrapper::wrap;

/// The (first iteration) thread pool implementation.
pub mod runner;

/// Run an experiment locally, as specified in the config file.
pub async fn run_local(
    experiment: &mut Experiment,
    exp_path: &Path,
    fs: &impl FileOperations,
    force: bool,
    sequential: bool,
) -> Result<()> {
    let cmds = wrap(experiment, exp_path, env::consts::ARCH, fs)?;
    trace!("Running cmds {:#?}", cmds);

    experiment.save(fs)?;

    run_locally(cmds, force, sequential).await?;

    Ok(())
}
