// The architecture of our codebase, shared between wrapper and CLI

/// A struct and related methods for global configuration,
/// declaratively specifying experiments.
pub mod config;

/// Code shared between the wrapper and `gourd`.
pub mod measurement;

/// The setup of an experiment.
pub mod experiment;

/// Common file operations
pub mod file_system;

/// The error handling for `gourd`.
pub mod error;

/// Constant values.
pub mod constants;

/// Afterscript definitions.
pub mod afterscript;