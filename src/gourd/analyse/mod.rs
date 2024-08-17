use std::cmp::max;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io::Write;
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
#[derive(Debug, Clone)]
pub struct Table<R: ToString + AsRef<[u8]>, const N: usize> {
    /// CSV-style table header.
    pub header: Option<[R; N]>,
    /// The table entries (= rows).
    pub body: Vec<[R; N]>,
    /// An optional footer, can be used to aggregate statistics, for example.
    pub footer: Option<[R; N]>,
}

impl<R: ToString + AsRef<[u8]>, const N: usize> Table<R, N> {
    /// Get the width (in utf-8 characters) of the longest entry of each column
    pub fn column_widths(&self) -> [usize; N] {
        let mut col_widths = [0usize; N];

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

    /// Write this table to a [`csv::Writer`]
    pub fn write_csv<W: Write>(&self, writer: &mut Writer<W>) -> Result<()> {
        if let Some(h) = &self.header {
            writer.write_record(h)?;
        }

        for row in &self.body {
            writer.write_record(row)?;
        }

        if let Some(f) = &self.footer {
            writer.write_record(f)?;
        }

        Ok(())
    }
}

impl<R: ToString + AsRef<[u8]>, const N: usize> Display for Table<R, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let col_widths = self.column_widths();
        if let Some(header) = &self.header {
            for (width, value) in col_widths.iter().zip(header.iter()) {
                write!(f, "| {: <width$} ", value.to_string())?;
            }
            writeln!(f, "|")?;

            for width in col_widths.iter() {
                write!(f, "*-{}-", "-".repeat(*width))?;
            }
            writeln!(f, "*")?;
        }

        for row in self.body.iter() {
            for (width, value) in col_widths.iter().zip(row.iter()) {
                write!(f, "| {: <width$} ", value.to_string())?;
            }
            writeln!(f, "|")?;
        }

        if let Some(footer) = &self.footer {
            for width in col_widths.iter() {
                write!(f, "*-{}-", "-".repeat(*width))?;
            }
            writeln!(f, "*")?;

            for (width, value) in col_widths.iter().zip(footer.iter()) {
                write!(f, "| {: <width$} ", value.to_string())?;
            }
            writeln!(f, "|")?;
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
