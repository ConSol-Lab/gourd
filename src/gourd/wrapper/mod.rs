/// Binary verification for linux.
mod check_binary_linux;
/// Binary verification for macos.
mod check_binary_macos;

use std::path::Path;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use std::path::PathBuf;
/// Verify if the architecture of a `binary` matched the `expected`
/// architecture.
use std::process::Command;

use anyhow::Result;
use gourd_lib::experiment::Experiment;
use gourd_lib::file_system::FileOperations;

use crate::status::DynamicStatus;
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
    experiment_path: &Path,
    arch: &str,
    fs: &impl FileOperations,
) -> Result<Vec<Command>> {
    let mut result = Vec::new();

    let mut runs_to_iterate = experiment.runs.clone();

    let status = experiment.status(fs)?;

    for (run_id, run) in runs_to_iterate.iter_mut().enumerate() {
        if status[&run_id].is_scheduled() || status[&run_id].is_completed() {
            continue; // i assume we ignore the scenario where a run is locally
                      // still running? (for example
                      // multiple terminals in the same gourd experiment)
        }

        let program = &experiment.get_program(run)?;

        verify_arch(&program.binary, arch, fs)?;

        let mut cmd = Command::new(&experiment.wrapper);

        cmd.arg(experiment_path).arg(format!("{}", run_id));

        result.push(cmd);
    }

    experiment.runs = runs_to_iterate;

    Ok(result)
}

// #[cfg(test)]
// #[path = "tests/mod.rs"]
// mod tests;
