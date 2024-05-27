use anstyle::AnsiColor;
use anyhow::Context;
use anyhow::Result;
use clap::crate_authors;
use clap::crate_name;
use clap::crate_version;
use gourd_lib::constants::style_from_fg;
use gourd_lib::constants::ERROR_STYLE;
use gourd_lib::constants::HELP_STYLE;
use gourd_lib::constants::LOGO;
use gourd_lib::constants::NAME_STYLE;
use gourd_lib::constants::PRIMARY_STYLE;
use gourd_lib::ctx;
use gourd_lib::error::Ctx;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;

/// Util function for getting the style for the CLI
#[cfg(not(tarpaulin_include))]
pub fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(style_from_fg(AnsiColor::Yellow).bold())
        .header(style_from_fg(AnsiColor::Green).bold().underline())
        .literal(style_from_fg(AnsiColor::Cyan).bold())
        .invalid(style_from_fg(AnsiColor::Blue).bold())
        .error(ERROR_STYLE)
        .valid(HELP_STYLE)
        .placeholder(style_from_fg(AnsiColor::White))
}

/// Pretty print gourd's version
#[cfg(not(tarpaulin_include))]
pub fn print_version(script: bool) {
    if script {
        println!("{} {}", crate_name!(), crate_version!());

        return;
    }

    let mut to_print = LOGO.replace(
        "{LINE1}",
        &format!(
            "  at version {}{}{:#}",
            PRIMARY_STYLE,
            crate_version!(),
            PRIMARY_STYLE
        ),
    );

    to_print = to_print.replace(
        "{LINE2}",
        &format!(
            "{}Technische Universiteit Delft 2024{:#}",
            NAME_STYLE, NAME_STYLE,
        ),
    );

    to_print = to_print.replace("{LINE3}", "Authored by:");

    for (idx, author) in crate_authors!("\n").split('\n').enumerate() {
        to_print = to_print.replace(&format!("{{LINE{}}}", idx + 4), author);
    }

    print!("{to_print}");
}

/// Util function: formatting a table for printing
///
/// input: Vec of rows, each row is a Vec of strings (columns)
///
/// output: String
pub fn format_table(data: Vec<Vec<String>>) -> String {
    if data.is_empty() {
        return String::new();
    }
    let mut max_widths = vec![0; data[0].len()];
    for row in &data {
        for (i, item) in row.iter().enumerate() {
            max_widths[i] = max_widths[i].max(item.len());
        }
    }
    let mut result = String::new();
    for row in data {
        let formatted_row: Vec<String> = row
            .into_iter()
            .enumerate()
            .map(|(i, item)| format!("{:width$}", item, width = max_widths[i]))
            .collect();
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&formatted_row.join(" | "));
    }
    result.trim().to_string()
}

/// Generates the progress bar used by the cli.
pub fn generate_progress_bar(len: u64) -> Result<ProgressBar> {
    let prog_style = ProgressStyle::with_template(
        "{prefix}[{spinner:.green}] {bar:.green/blue} {msg} {pos}/{len}",
    )
    .with_context(ctx!("Failed to create the progress bar",;"",))?
    .progress_chars("##-");

    let bar = ProgressBar::new(len);
    bar.set_message("Running jobs...");
    bar.set_style(prog_style);

    Ok(bar)
}

#[cfg(test)]
#[path = "tests/printing.rs"]
mod tests;
