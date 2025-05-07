/// Binary verification for linux.
mod check_binary_linux;
/// Binary verification for macos.
mod check_binary_macos;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use std::path::PathBuf;
/// Verify if the architecture of a `binary` matched the `expected`
/// architecture.
use std::process::Command;

use anyhow::Result;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;
use log::trace;

use crate::chunks::Chunkable;
use crate::status::ExperimentStatus;
#[cfg(target_os = "linux")]
use crate::wrapper::check_binary_linux::verify_arch;
#[cfg(target_os = "macos")]
use crate::wrapper::check_binary_macos::verify_arch;

/// Verify the architecture of the binary.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn verify_arch(_: &PathBuf, _: &str, _: &impl FileOperations) -> Result<()> {
    Ok(())
}

/// This function returns the commands to be run for an n x m matching of the
/// runs to tests.
///
/// The results and outputs will be located in `config.output_dir`.
pub fn wrap(
    experiment: &mut Experiment,
    status: &ExperimentStatus,
    arch: &str,
    fs: &impl FileOperations,
) -> Result<Vec<Command>> {
    let mut result = Vec::new();

    let binding = experiment.clone();
    let runs_to_iterate = binding.unscheduled(status);
    let chunk_index =
        experiment.register_runs(&runs_to_iterate.iter().map(|(v, _)| *v).collect::<Vec<_>>());

    trace!("There are {} unscheduled runs", runs_to_iterate.len());

    for (task_id, run) in runs_to_iterate.into_iter().map(|(_, r)| r).enumerate() {
        let program = &experiment.get_program(run)?;

        verify_arch(&program.binary, arch, fs)?;

        let mut cmd = Command::new(shellexpand::tilde(&experiment.wrapper).to_string());

        cmd.arg(experiment.file())
            .arg(format!("{chunk_index}"))
            .arg(format!("{task_id}"));

        result.push(cmd);
    }

    Ok(result)
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
