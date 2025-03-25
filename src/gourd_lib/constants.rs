use std::path::PathBuf;
use std::time::Duration;

use anstyle::AnsiColor;
use anstyle::Color;
use anstyle::Color::Ansi;
use anstyle::Style;

use crate::config::slurm::ResourceLimits;

/// The version name for Gourd!
/// Ensure it matches:
/// - Cargo.toml
/// - docs/maintainer/version-history/section.tex
/// - docs/user/gourd-tutorial.7.tex
/// - docs/user/gourd.1.tex
/// - docs/user/gourd.toml.5.tex
pub const GOURD_VERSION: &str = "Sponge Gourd";

/// The default path to the wrapper, that is, we assume `gourd_wrapper` is in
/// $PATH.
pub const WRAPPER_DEFAULT: fn() -> String = || "gourd_wrapper".to_string();

/// The default path to the afterscript.
pub const AFTERSCRIPT_DEFAULT: fn() -> Option<PathBuf> = || None;

/// The default postprocess job name.
pub const POSTPROCESS_JOB_DEFAULT: fn() -> Option<String> = || None;

/// The default value of resource limits for a program.
pub const PROGRAM_RESOURCES_DEFAULT: fn() -> Option<ResourceLimits> = || None;

/// The default value of warning on label overlaps.
pub const LABEL_OVERLAP_DEFAULT: fn() -> bool = || false;

/// The default arguments for an input.
pub const EMPTY_ARGS: fn() -> Vec<String> = Vec::new;

/// The prefix which will cause an argument to be interpreted as a glob.
/// Ensure matches:
/// - docs/user/gourd.toml.5
pub const GLOB_ESCAPE: &str = "path|";

/// The prefix which will cause an argument to be interpreted as a parameter.
/// Ensure matches:
/// - docs/user/gourd.toml.5
pub const PARAMETER_ESCAPE: &str = "param|";

/// The prefix which will cause an argument to be interpreted as a subparameter.
/// Ensure matches:
/// - docs/user/gourd.toml.5
pub const SUB_PARAMETER_ESCAPE: &str = "subparam|";

/// The internal representation of inputs generated from a schema
pub const INTERNAL_SCHEMA_INPUTS: &str = "schema";

/// The internal representation of globbed inputs.
pub const INTERNAL_GLOB: &str = "glob";

/// The internal representation of paramtrized inputs.
pub const INTERNAL_PARAMETER: &str = "param";

/// Internal representation for names parsed from config
pub const INTERNAL_PREFIX: &str = "_i_";

/// The amount between refreshes of the status screen, in ms.
pub const STATUS_REFRESH_PERIOD: Duration = Duration::from_millis(500);

/// Create a style with a defined foreground color.
pub const fn style_from_fg(color: AnsiColor) -> Style {
    Style::new().fg_color(Some(Color::Ansi(color)))
}

/// The styling for the program name.
pub const PRIMARY_STYLE: Style = style_from_fg(AnsiColor::Green).bold();

/// The styling for the secondary text.
pub const SECONDARY_STYLE: Style = style_from_fg(AnsiColor::BrightGreen);

/// The styling for the tertiary text.
pub const TERTIARY_STYLE: Style = style_from_fg(AnsiColor::Blue);

/// The styling for the university name.
pub const NAME_STYLE: Style = Style::new().bold();

/// The styling for error messages.
pub const ERROR_STYLE: Style = style_from_fg(AnsiColor::Red).bold();

/// The styling for warning messages.
pub const WARNING_STYLE: Style = style_from_fg(AnsiColor::Yellow).bold();

/// The styling for help messages.
pub const HELP_STYLE: Style = style_from_fg(AnsiColor::Green).bold().underline();

/// Style of commands in doc messages
pub const CMD_DOC_STYLE: Style = Style::new()
    .italic()
    .bg_color(Some(Ansi(AnsiColor::BrightBlue)))
    .fg_color(Some(Ansi(AnsiColor::Black)));

/// Style of commands in help messages
pub const CMD_STYLE: Style = Style::new()
    .bold()
    .bg_color(Some(Ansi(AnsiColor::Green)))
    .fg_color(Some(Ansi(AnsiColor::Black)));

/// Style of [`std::path::Path`]s and [`PathBuf`]s
pub const PATH_STYLE: Style = Style::new()
    .italic()
    .fg_color(Some(Ansi(AnsiColor::BrightBlue)));

/// Grayed paragraphs of not very important text
pub const PARAGRAPH_STYLE: Style = Style::new().fg_color(Some(Ansi(AnsiColor::BrightBlack)));

/// Supported SLURM versions.
pub const SLURM_VERSIONS: [[u64; 2]; 1] = [[21, 8]];

/// Possible values for Mail Type in slurm configuration
pub const MAIL_TYPE_VALID_OPTIONS: [&str; 13] = [
    "NONE",
    "BEGIN",
    "END",
    "FAIL",
    "REQUEUE",
    "ALL",
    "INVALID_DEPEND",
    "STAGE_OUT",
    "TIME_LIMIT",
    "TIME_LIMIT_90",
    "TIME_LIMIT_80",
    "TIME_LIMIT_50",
    "ARRAY_TASKS",
];

/// The maximal amount of runs before status only shows a short summary.
pub const SHORTEN_STATUS_CUTOFF: usize = 40;

/// The max (bytes, lines) of afterscript output that will be displayed in gourd
/// status of a run
pub const TRUNCATE_AFTERSCRIPT_OUTPUT: (usize, usize) = (500, 20);

/// Maximal number of individual prompts that the user can be asked when trying
/// to rerun
pub const RERUN_LIST_PROMPT_CUTOFF: usize = 15;

/// Do we assume by default that runs with custom labels are failed runs?
pub const RERUN_LABEL_BY_DEFAULT: fn() -> bool = || true;

/// The maximal amount of tasks that gourd will schedule.
pub const TASK_LIMIT: usize = 200;

/// The logo of the application.
pub const LOGO: &str = include_str!("../resources/logo.ascii");

/// The length of the bar for scheduling.
pub const SCHEDULE_BAR_WIDTH: usize = 50;

/// The size of the analysis output plots, in pixels.
pub const PLOT_SIZE: (u32, u32) = (1920, 1080);
