use std::path::PathBuf;

use clap::ArgAction;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

/// Structure of the main command (gourd).
#[allow(unused)]
#[derive(Parser, Debug)]
#[command(
    about = "Gourd, an empirical evaluator",
    disable_help_subcommand = true,
    version
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

    /// Verbose mode, prints debug info. For even more try: -vv.
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

    /// Display full afterscript output for a run. Use with -i <run_id>
    #[arg(long, requires = "run_id")]
    pub after_out: bool,
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

    /// Plot analysis or create a table for the run metrics.
    #[command(subcommand)]
    pub subcommand: AnalyseSubcommand,

    /// If you want to save to a specific file
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// Enum for subcommands of the `run` subcommand.
#[derive(Subcommand, Debug, Clone)]
pub enum AnalyseSubcommand {
    /// Generate a cactus plot for the runs of this experiment.
    #[command()]
    Plot {
        /// What file format to make the cactus plot in.
        /// Options are `png` (default), `svg`
        #[arg(short, long, default_value = "png")]
        format: PlotType,

        /// If you want to save to a specific file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate tables for the metrics of the runs in this experiment.
    #[command()]
    Table(CsvFormatting),
}

/// Construct a CSV by specifying desired columns and any grouping of runs.
#[derive(Args, Debug, Clone)]
pub struct CsvFormatting {
    /// Group together the averages based on a number of conditions.
    ///
    /// Specifying multiple conditions means that all equalities must hold.
    #[arg(short, long, value_delimiter = ',', num_args = 0..)]
    pub group: Vec<GroupBy>,

    /// Choose which columns to include in the table.
    #[arg(short, long, value_delimiter = ',', num_args = 1..)]
    pub format: Option<Vec<CsvColumn>>,

    /// If you want to save to a specific file
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// Choice of grouping together runs based on equality conditions
#[derive(ValueEnum, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub enum GroupBy {
    /// Group together runs that have the same program.
    Program,
    /// Group together runs that have the same input.
    Input,
    /// Group together runs that have the same input group.
    Group,
}

/// Enum for the columns that can be included in the CSV.
#[derive(ValueEnum, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub enum CsvColumn {
    /// The name of the program that was run.
    Program,
    /// The input file that was used.
    File,
    /// The arguments that were passed to the program.
    Args,
    /// The group that the run was in.
    Group,
    /// The afterscript that was run.
    Label,
    /// The afterscript output content.
    Afterscript,
    /// The slurm completion status of the run.
    Slurm,
    /// The metrics saved file for the run
    FsStatus,
    /// The run process exit code
    ExitCode,
    /// Process wall time
    WallTime,
    /// Process user time
    UserTime,
    /// Process system time
    SystemTime,
    /// Maximum resident set size
    MaxRSS,
    /// Integral shared memory size
    IxRSS,
    /// Integral unshared data size
    IdRSS,
    /// Integral unshared stack size
    IsRSS,
    /// Page reclaims (soft page faults)
    MinFlt,
    /// Page faults (hard page faults)
    MajFlt,
    /// Swaps
    NSwap,
    /// Block input operations
    InBlock,
    /// Block output operations
    OuBlock,
    /// IPC messages sent
    MsgSent,
    /// IPC messages received
    MsgRecv,
    /// Signals received
    NSignals,
    /// Voluntary context switches
    NVCsw,
    /// Involuntary context switches
    NIvCsw,
}

/// Enum for the output format of the analysis.
#[derive(ValueEnum, Debug, Clone, Default, Copy)]
pub enum PlotType {
    /// Output an SVG cactus plot.
    Svg,

    /// Output a PNG cactus plot.
    #[default]
    Png,
}

impl PlotType {
    /// get the file extension for this plot type
    pub fn ext(&self) -> &str {
        match self {
            PlotType::Svg => "svg",
            PlotType::Png => "png",
        }
    }
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
