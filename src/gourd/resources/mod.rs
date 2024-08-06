use std::path::Path;
use std::process::Command;
use std::process::ExitStatus;

use anyhow::Context;
use anyhow::Result;
use gourd_lib::ctx;

/// Runs a shell script.
pub fn run_script(arguments: Vec<&str>, work_dir: &Path) -> Result<ExitStatus> {
    let mut command = Command::new("sh");

    command.args(&arguments);
    command.current_dir(work_dir);

    command
        .spawn()
        .with_context(ctx!("Could not spawn child sh {arguments:?}", ; "",))?
        .wait()
        .with_context(ctx!("Could not wait for script sh {arguments:?}", ; "",))
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
