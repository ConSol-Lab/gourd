use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::process::Output;

use anyhow::Context;
use anyhow::Result;
use log::trace;

use crate::ctx;

/// Runs a shell script.
pub fn run_script<T>(cmd: T, arguments: Vec<&str>, work_dir: &Path) -> Result<Output>
where
    T: AsRef<OsStr>,
{
    let mut command = Command::new(cmd);

    command.args(&arguments);
    command.current_dir(work_dir);

    trace!("Running script: {command:?}");

    command
        .output()
        .with_context(ctx!("Could not spawn child {command:?}", ; "",))
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
