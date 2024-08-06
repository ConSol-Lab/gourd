use std::io;
use std::process;
use std::process::Command;
use std::process::Output;

use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::constants::NAME_STYLE;
use gourd_lib::constants::PRIMARY_STYLE;
use gourd_lib::constants::TASK_LIMIT;
use log::error;
use log::trace;
use tokio::task::JoinSet;

/// Run a list of tasks locally in a multithreaded way.
pub async fn run_locally(tasks: Vec<Command>, force: bool, sequential: bool) -> Result<()> {
    if tasks.len() > TASK_LIMIT && !force && !sequential {
        bailc!(
          "task limit exceeded", ;
          "{PRIMARY_STYLE}gourd{PRIMARY_STYLE:#} will not run more than \
          {TASK_LIMIT} jobs on local, doing so may possibly exhaust your file descriptors", ;
          "if you are {NAME_STYLE}absolutely{NAME_STYLE:#} sure that you \
          want to run {} tasks use the {PRIMARY_STYLE}--force{PRIMARY_STYLE:#} \
          option", tasks.len()
        )
    }

    #[cfg(not(tarpaulin_include))] // Tarpaulin can't calculate the coverage correctly
    tokio::spawn(async move {
        /// Error in case of wrapper failure.
        fn handle_output(join: io::Result<Output>) {
            if let Ok(exit) = join {
                if !exit.status.success() {
                    error!("Failed to run gourd wrapper: {:?}", exit.status);
                    error!(
                        "Wrapper returned: {}",
                        String::from_utf8(exit.stderr).unwrap()
                    );
                    process::exit(1);
                }
            } else {
                error!("Couldn't start the wrapper: {join:?}");
                error!("Ensure that the wrapper is accesible. (see man gourd)");
                process::exit(1);
            }
        }

        if sequential {
            for mut task in tasks {
                trace!("Running task: {:?}", task);
                handle_output(task.output());
            }
        } else {
            let mut set = JoinSet::new();

            for mut task in tasks {
                trace!("Queueing task: {:?}", task);
                set.spawn_blocking(move || task.output());
            }

            while let Some(result) = set.join_next().await {
                if let Ok(join) = result {
                    handle_output(join);
                } else {
                    error!("Could not join the child in the multithreaded runtime");
                    process::exit(1);
                }
            }
        }

        Result::<()>::Ok(())
    });

    Ok(())
}

#[cfg(test)]
#[path = "tests/runner.rs"]
mod tests;
