use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::process::ExitStatus;

use anyhow::Context;
use anyhow::Result;
use gourd_lib::ctx;
use gourd_lib::error::Ctx;
use log::trace;

/// Runs a shell script.
pub fn run_script<T>(cmd: T, arguments: Vec<&str>, work_dir: &Path) -> Result<ExitStatus>
where
    T: AsRef<OsStr>,
{
    let mut command = Command::new(cmd);

    command.args(&arguments);
    command.current_dir(work_dir);

    trace!("Running script: {:?}", command);

    command
        .status()
        .with_context(ctx!("Could not spawn child {command:?}", ; "",))
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
