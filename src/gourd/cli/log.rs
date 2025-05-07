use anstyle::AnsiColor;
use colog::format::CologStyle;
use gourd_lib::constants::style_from_fg;
use log::Level;

/// Defines the logging tokens for `colog`.
#[derive(Debug, Clone, Copy)]
pub struct LogTokens;

// It does not make sense to test this impl
impl CologStyle for LogTokens {
    fn level_token(&self, level: &Level) -> &str {
        match *level {
            Level::Error => "error",
            Level::Warn => "warn",
            Level::Info => "info",
            Level::Debug => "debug",
            Level::Trace => "trace",
        }
    }

    fn prefix_token(&self, level: &Level) -> String {
        format!("{}:", self.level_color(level, self.level_token(level)),)
    }

    fn level_color(&self, level: &log::Level, msg: &str) -> String {
        let style = match level {
            Level::Error => style_from_fg(AnsiColor::Red),
            Level::Warn => style_from_fg(AnsiColor::Yellow),
            Level::Info => style_from_fg(AnsiColor::Green),
            Level::Debug => style_from_fg(AnsiColor::Blue),
            Level::Trace => style_from_fg(AnsiColor::Magenta),
        };

        format!("{style}{msg}{style:#}")
    }
}
