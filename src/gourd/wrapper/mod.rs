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
use gourd_lib::experiment::Run;
use gourd_lib::file_system::FileOperations;

use crate::chunks::Chunkable;
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

    let status = experiment.status(fs)?;

    let exp = experiment.clone();
    let runs_to_iterate: Vec<&mut Run> = experiment.unscheduled_mut(&status);

    for (run_id, run) in runs_to_iterate.into_iter().enumerate() {
        let program = &exp.get_program(run)?;

        verify_arch(&program.binary, arch, fs)?;

        let mut cmd = Command::new(&exp.wrapper);

        cmd.arg(experiment_path).arg(format!("{}", run_id));

        result.push(cmd);

        run.slurm_id = Some(String::default());
    }

    Ok(result)
}

// #[cfg(test)]
// #[path = "tests/mod.rs"]
// mod tests;
