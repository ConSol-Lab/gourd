use std::cmp::max;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use csv::Writer;
use gourd_lib::bailc;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::FieldRef;

use crate::status::FsState;
use crate::status::Status;

/// Export experiment data as CSV file
pub mod csvs;
/// Draw up plots of experiment data
pub mod plotting;

/// Represent a human-readable table.
/// Universal between CSV exporting and in-line display.
///
/// Since tables store the display strings, their entries are in essence
/// immutable. Cells are not meant to be read or modified, since that would
/// likely involve parsing the number in it, which is just unhygienic.
///
/// You can append rows to a table with [`Table::append_column`],
/// or create new columns using [`ColumnGenerator`]s.
#[derive(Debug, Clone)]
pub struct Table {
    /// Number of columns in the table.
    pub columns: usize,
    /// CSV-style table header.
    pub header: Option<Vec<String>>,
    /// The table entries (vector of rows, each row is a vector of entries)
    /// (`Vec<Row<Entry>>`).
    pub body: Vec<Vec<String>>,
    /// An optional footer, can be used to aggregate statistics, for example.
    pub footer: Option<Vec<String>>,
}

/// A column that can be appended to the end of a [`Table`].
///
/// Intended to be created through a [`ColumnGenerator`].
#[derive(Debug, Clone)]
pub struct Column {
    /// The text header of the column. Defaults to empty string
    pub header: Option<String>,
    /// The row cells of this column
    pub body: Vec<String>,
    /// The footer cell of this column. Defaults to empty string.
    pub footer: Option<String>,
}

/// Create a [`Column`] from a list of entries of type `X`.
#[derive(Debug, Clone)]
pub struct ColumnGenerator<X> {
    /// The text header of the column. Defaults to empty string
    pub header: Option<String>,
    /// A function to convert a type `X` element into the content of its
    /// equivalent row in the column body.
    pub body: fn(&Experiment, &X) -> Result<String>,
    /// A footer cell that can hold info aggregated
    /// from all the entries in the original list.
    pub footer: fn(&Experiment, &[X]) -> Result<Option<String>>,
}

impl<X> ColumnGenerator<X> {
    /// Generate a column from a vector of entries.
    pub fn generate(&self, exp: &Experiment, from: &[X]) -> Result<Column> {
        Ok(Column {
            header: self.header.clone(),
            body: from
                .iter()
                .map(|x| (self.body)(exp, x))
                .collect::<Result<Vec<String>>>()?,
            footer: (self.footer)(exp, from)?,
        })
    }
}

impl Table {
    /// Get the width (in utf-8 characters) of the longest entry of each column
    pub fn column_widths(&self) -> Vec<usize> {
        let mut col_widths = vec![0; self.columns];

        for row in self
            .header
            .iter()
            .chain(self.body.iter())
            .chain(self.footer.iter())
        {
            for (i, x) in col_widths
                .clone()
                .iter()
                .zip(row.iter().map(|x| x.to_string().chars().count()))
                .map(|(a, b)| *max(a, &b))
                .enumerate()
            {
                col_widths[i] = x;
            }
        }

        col_widths
    }

    /// Write this table to a [`Writer`]
    pub fn write_csv<W: Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        if let Some(h) = &self.header {
            writer.write_record(h)?;
        }

        for row in &self.body {
            writer.write_record(row)?;
        }

        // the footer is omitted in csv output to make analysis easier.

        Ok(())
    }

    /// Write this table to a file at the given path.
    pub fn write_to_path(&self, path: &Path) -> Result<()> {
        let mut writer = Writer::from_path(path).context("Failed to open file for writing")?;
        self.write_csv(&mut writer)?;
        writer.flush()?;
        Ok(())
    }

    /// Append a column to the table.
    // Known issue: https://github.com/rust-lang/rust-clippy/issues/13185
    #[allow(clippy::manual_inspect)]
    pub fn append_column(&mut self, column: Column) {
        self.columns += 1;
        self.header = self
            .header
            .as_mut()
            .map(|h| {
                h.push(column.header.clone().unwrap_or_default());
                h
            })
            .cloned();
        debug_assert_eq!(self.body.len(), column.body.len());
        self.body = self
            .body
            .iter_mut()
            .zip(column.body.iter())
            .map(|(a, b)| {
                a.push(b.clone());
                a.clone()
            })
            .collect();
        self.footer = self
            .footer
            .as_mut()
            .map(|f| {
                f.push(column.footer.clone().unwrap_or_default());
                f
            })
            .cloned();
    }
}

impl Display for Table {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.sign_minus() {
            // reduced output
            for row in self.body.iter() {
                writeln!(f)?;
                for value in row.iter() {
                    write!(f, "{value}\t")?;
                }
            }
        } else {
            writeln!(f)?;
            let col_widths = self.column_widths();
            if let Some(header) = &self.header {
                for (width, value) in col_widths.iter().zip(header.iter()) {
                    write!(f, "| {value: <width$} ")?;
                }
                writeln!(f, "|")?;

                for width in col_widths.iter() {
                    write!(f, "*-{}-", "-".repeat(*width))?;
                }
                writeln!(f, "*")?;
            }

            for row in self.body.iter() {
                for (width, value) in col_widths.iter().zip(row.iter()) {
                    write!(f, "| {value: <width$} ")?;
                }
                writeln!(f, "|")?;
            }

            if let Some(footer) = &self.footer {
                for width in col_widths.iter() {
                    write!(f, "*-{}-", "-".repeat(*width))?;
                }
                writeln!(f, "*")?;

                for (width, value) in col_widths.iter().zip(footer.iter()) {
                    write!(f, "| {value: <width$} ")?;
                }
                writeln!(f, "|")?;
            }
        }

        Ok(())
    }
}

/// Get completion times of jobs.
pub fn get_completions(
    statuses: BTreeMap<usize, Status>,
    experiment: &Experiment,
) -> Result<BTreeMap<FieldRef, Vec<u128>>> {
    let mut completions: BTreeMap<FieldRef, Vec<u128>> = BTreeMap::new();

    for (id, status) in statuses {
        let program_name = experiment.program_from_run_id(id)?.name;

        if status.is_completed() {
            let time = match get_completion_time(status.fs_status.completion) {
                Ok(t) => t.as_nanos(),
                // No RUsage
                Err(_) => continue,
            };

            if completions.contains_key(&program_name) {
                let mut times = completions[&program_name].clone();
                times.push(time);
                completions.insert(program_name.clone(), times);
            } else {
                completions.insert(program_name.clone(), vec![time]);
            }
        }
    }

    for times in completions.values_mut() {
        times.sort();
    }
    Ok(completions)
}

/// Get completion time of a run.
pub fn get_completion_time(state: FsState) -> Result<Duration> {
    match state {
        FsState::Completed(measured) => {
            let measured = measured.rusage;

            if let Some(r) = measured {
                Ok(r.utime)
            } else {
                bailc!("RUsage is not accessible even though the run completed");
            }
        }
        _ => {
            bailc!("Run was supposed to be completed");
        }
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
