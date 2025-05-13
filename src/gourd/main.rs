//! # `gourd`
//!
//! **`gourd`** is a command-line tool that schedules parallel runs for
//! algorithm comparisons.
//!
//! Given the parameters of the experiment, a number of test datasets,
//! and algorithm implementations to compare, `gourd` runs the experiment in
//! parallel and provides many options for processing its results.
//!
//! While originally envisioned for the DelftBlue supercomputer at
//! Delft University of Technology, `gourd` can replicate the experiment on
//! any cluster computer with the _Slurm_ scheduler, on any UNIX-like system,
//! and on Microsoft Windows.
//!
//!
//! # Usage
//!
//! Verify that `gourd` is installed by running:
//! ```
//! $ gourd version
//! ```
//!
//! Congratulations! Now you can use `gourd`.
//!
//! There is extensive tutorials and documentation which can be accessed in many
//! different formats.
//!
//! As a manpage, with the `man` command:
//! ```
//! $ man gourd
//! $ man gourd.toml
//! $ man gourd-tutorial
//! ```

/// An interface for communicating with `gourd-wrapper`, a separately
/// packaged binary that encapsulates the user's programs.
pub mod wrapper;

/// A framework for running experiments on the local machine using a
/// thread-pool executor.
pub mod local;

/// A framework for running supercomputer experiments by interfacing
/// with a local installation of SLURM.
pub mod slurm;

/// Functionality for user-friendly initialisation of new experimental
/// setups and examples.
pub mod init;

/// The command line interface and relevant structures.
pub mod cli;

/// Functionality for checking and displaying the status of already
/// running experiments.
pub mod status;

/// Extensions to the `Experiment` struct defined in `gourd-lib`,
/// allowing for operations on runtime data.
pub mod experiments;

/// Functionality for post-processing jobs including after-scripts,
/// pipeline jobs, and retrieval of their status.
pub mod post;

/// Rerun subcommand helper functions
pub mod rerun;

/// An implementation for allocating queued jobs to chunks
pub mod chunks;

/// Analysing runs - collecting metrics, exporting, plotting.
pub mod analyse;

/// Convenience functions for unit tests.
#[cfg(test)]
pub mod test_utils;

/// The main CLI entry-point of the `gourd` utility.
///
/// This function parses command-line arguments and executes
/// sub-commands as specified by the user.
#[tokio::main]
async fn main() {
    cli::process::parse_command().await;
}
