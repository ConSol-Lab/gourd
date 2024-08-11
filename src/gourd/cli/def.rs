use std::path::PathBuf;
use std::time::Duration;

use clap::builder::PossibleValue;
use clap::ArgAction;
use clap::Args;
use clap::Parser;
use clap::Subcommand;

/// Structure of the main command (gourd).
#[allow(unused)]
#[derive(Parser, Debug)]
#[command(
    about = "Gourd, an empirical evaluator",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// The main command issued.
    #[command(subcommand)]
    pub command: GourdCommand,

    /// Disable interactive mode, for use in scripts.
    #[arg(short, long, global = true)]
    pub script: bool,

    /// The path to the config file.
    #[arg(short, long, default_value = "./gourd.toml", global = true)]
    pub config: PathBuf,

    /// Verbose mode, displays debug info. For even more try: -vv.
    #[arg(short, long, global = true, action = ArgAction::Count)]
    pub verbose: u8,

    /// Dry run, run but don't actually affect anything.
    #[arg(short, long, global = true)]
    pub dry: bool,
}

/// Arguments supplied with the `run` command.
#[derive(Args, Debug, Clone, Copy)]
pub struct RunStruct {
    /// The run mode of this run.
    #[command(subcommand)]
    pub subcommand: RunSubcommand,
}

/// Enum for subcommands of the `run` subcommand.
#[derive(Subcommand, Debug, Copy, Clone)]
pub enum RunSubcommand {
    /// Create and run an experiment on this computer.
    #[command()]
    Local {
        /// Force running more experiments than recommended.
        #[arg(long)]
        force: bool,

        /// Force running the experiments in sequence rather than concurrently.
        #[arg(long)]
        sequential: bool,
    },

    /// Create and run an experiment using Slurm.
    #[command()]
    Slurm {},
}

/// Arguments for the Rerun command.
#[derive(Args, Debug, Clone)]
pub struct RerunOptions {
    /// The id of the experiment to rerun jobs for
    /// [default: newest experiment]
    #[arg(value_name = "EXPERIMENT")]
    pub experiment_id: Option<usize>,

    /// The ids of the runs to rerun [default: all failed runs]
    #[clap(short, long, value_delimiter = ' ', num_args = 1..)]
    pub run_ids: Option<Vec<usize>>,
}

/// Arguments supplied with the `status` command.
#[derive(Args, Debug, Clone, Copy)]
pub struct StatusStruct {
    /// The id of the experiment for which to fetch status
    /// [default: newest experiment].
    #[arg(value_name = "EXPERIMENT")]
    pub experiment_id: Option<usize>,

    /// Get a detailed description of a run by providing its id.
    #[arg(short = 'i', long)]
    pub run_id: Option<usize>,

    /// Do not exit until all jobs are finished.
    #[arg(long)]
    pub follow: bool,

    /// Do not shorten output even if there is a lot of runs.
    #[arg(long)]
    pub full: bool,
}

/// Arguments supplied with the `continue` command.
#[derive(Args, Debug, Clone, Copy)]
pub struct ContinueStruct {
    /// The id of the experiment for which to fetch status
    /// [default: newest experiment].
    #[arg(value_name = "EXPERIMENT")]
    pub experiment_id: Option<usize>,
}

/// Structure of cancel subcommand.
#[derive(Args, Debug, Clone)]
pub struct CancelStruct {
    /// The id of the experiment of which to cancel runs
    /// [default: newest experiment]
    #[arg(value_name = "EXPERIMENT")]
    pub experiment_id: Option<usize>,

    /// Cancel specific runs by providing their run ids,
    /// for example: `gourd cancel -i 5` or `gourd cancel -i 1 2 3`.
    #[arg(short = 'i', long, value_delimiter = ' ', num_args = 1..)]
    pub run_ids: Option<Vec<usize>>,

    /// Cancel all the experiments from this account (not only those by gourd).
    /// To see what runs would be cancelled, use the `--dry` flag.
    #[arg(
        short,
        long,
        conflicts_with_all = ["experiment_id", "run_ids"],
    )]
    pub all: bool,
}

/// Arguments supplied with the `init` command.
#[derive(Args, Debug, Clone)]
pub struct InitStruct {
    /// The directory in which to initialise a new experimental setup.
    #[arg()]
    pub directory: Option<PathBuf>,

    /// The name of an example experiment in gourd-tutorial(7).
    #[arg(short, long)]
    pub example: Option<String>,

    /// List the available example options.
    #[arg(short, long)]
    pub list_examples: bool,

    /// Initialise a git repository for the setup.
    #[arg(
    long,
    action = ArgAction::Set,
    default_value_t = true,             // No flag evaluates to true.
    default_missing_value = "true",     // "--git" evaluates to true.
    num_args = 0..=1,                   // "--git=true" evaluates to true.
    require_equals = false,             // "--git=false" evaluates to false.
    )]
    pub git: bool,
}

/// Arguments supplied with the `analyse` command.
#[derive(Args, Debug, Clone)]
pub struct AnalyseStruct {
    /// The id of the experiment to analyse
    /// [default: newest experiment].
    #[arg(value_name = "EXPERIMENT")]
    pub experiment_id: Option<usize>,

    /// The output format of the analysis.
    /// For all formats see the manual.
    #[arg(long, short, default_value = "csv", value_parser = [
        PossibleValue::new("csv"),
        PossibleValue::new("plot-svg"),
        PossibleValue::new("plot-png"),
    ])]
    pub output: String,
}

/// Arguments supplied with the `set-limits` command.
#[derive(Args, Debug, Clone)]
pub struct SetLimitsStruct {
    /// The id of the experiment of which to change limits
    /// [default: newest experiment]
    #[arg(value_name = "EXPERIMENT")]
    pub experiment_id: Option<usize>,

    /// The program for which to set resource limits.
    #[arg(short, long)]
    pub program: Option<String>,

    /// Set resource limits for all programs.
    #[arg(
        short,
        long,
        conflicts_with_all = ["program"],
    )]
    pub all: bool,

    /// Take the resource limits from a toml file.
    #[arg(long)]
    pub mem: Option<usize>,

    /// Take the resource limits from a toml file.
    #[arg(long)]
    pub cpu: Option<usize>,

    /// Take the resource limits from a toml file.
    #[arg(long, value_parser = humantime::parse_duration)]
    pub time: Option<Duration>,
}

/// Enum for root-level `gourd` commands.
#[derive(Subcommand, Debug)]
pub enum GourdCommand {
    /// Create an experiment from configuration and run it.
    #[command()]
    Run(RunStruct),

    /// Set up a template of an experiment configuration.
    #[command()]
    Init(InitStruct),

    /// Display the status of an experiment that was run.
    #[command()]
    Status(StatusStruct),

    /// Schedule another batch of slurm jobs.
    #[command()]
    Continue(ContinueStruct),

    /// Cancel runs.
    #[command()]
    Cancel(CancelStruct),

    /// Rerun some of the runs from existing experiments
    #[command()]
    Rerun(RerunOptions),

    /// Output metrics of completed runs.
    #[command()]
    Analyse(AnalyseStruct),

    /// Print information about the version.
    #[command()]
    Version,
}
