use std::collections::BTreeMap;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::ctx;
use gourd_lib::experiment::Experiment;
use log::trace;
use regex_lite::Regex;

use super::SlurmBasedStatus;
use super::SlurmState::*;
use super::StatusProvider;
use crate::slurm::SlurmInteractor;

/// Structure of Sacct output.
#[derive(Debug, Clone, PartialEq)]
pub struct SacctOutput {
    /// ID of the job
    pub job_id: String,

    /// Name of the job
    pub job_name: String,

    /// Current state of the job
    pub state: String,

    /// Exit code of slurm
    pub slurm_exit_code: isize,

    /// Exit code of the program
    pub program_exit_code: isize,
}

/// Provide job status information based on the files system information.
#[derive(Debug, Clone, Copy)]
pub struct SlurmBasedProvider {}

impl<T> StatusProvider<T, SlurmBasedStatus> for SlurmBasedProvider
where
    T: SlurmInteractor,
{
    #[cfg(not(tarpaulin_include))]
    fn get_statuses(
        connection: &T,
        experiment: &Experiment,
    ) -> Result<BTreeMap<usize, SlurmBasedStatus>> {
        let mut run_id_to_status: BTreeMap<usize, SlurmBasedStatus> = BTreeMap::new();
        let mut slurm_map = BTreeMap::new();

        for (run_id, run) in experiment.runs.iter().enumerate() {
            slurm_map.insert(run.slurm_id.clone(), run_id);
        }

        trace!("Slurm map: {:?}", slurm_map);

        let statuses: Vec<SacctOutput> = flatten_job_id(
            connection.get_accounting_data(
                experiment
                    .runs
                    .iter()
                    .filter_map(|x| x.slurm_id.clone())
                    .collect(),
            )?,
        )?;

        for job in statuses {
            // Mapping of all possible job state codes
            // https://slurm.schedmd.com/sacct.html#SECTION_JOB-STATE-CODES
            let completion = match job
                .state
                .split(' ')
                .next()
                .ok_or(anyhow!("Failed to get completion status from sacct"))?
            {
                "BOOT_FAIL" | "BF" => BootFail,

                "CANCELLED" | "CA" => Cancelled,

                "COMPLETED" | "CO" => Success,

                "DEADLINE" | "DL" => Deadline,

                "FAILED" | "F" => SlurmFail,

                "NODE_FAIL" | "NF" => NodeFail,

                "OUT_OF_MEMORY" | "OOM" => OutOfMemory,

                "PENDING" | "PD" => Pending,

                "PREEMPTED" | "PR" => Preempted,

                "RUNNING" | "R" => Running,

                "REQUEUED" | "RQ" => Pending, // For now we treat it as pending,
                // but it may need its own label, for example State::Requeued
                "RESIZING" | "RS" => Running, // Needs a label, did not think of any suitable

                "REVOKED" | "RV" => Pending, // Also will probably need a label

                "SUSPENDED" | "S" => Suspended,

                "TIMEOUT" | "TO" => Timeout,

                // if not specified above we assume it failed
                _ => bailc!("Sacct returned unexpected output", ; "", ; "",),
            };

            if let Some(existing_run) = slurm_map.get(&Some(job.job_id.clone())) {
                trace!("run {existing_run} is {completion:?}");
                run_id_to_status.insert(
                    *existing_run,
                    SlurmBasedStatus {
                        completion,
                        exit_code_program: job.program_exit_code,
                        exit_code_slurm: job.slurm_exit_code,
                    },
                );
            } else {
                trace!("Sacct gave output {completion:?} for slurm job {job:?}");
                trace!(
                    "but it isn't a part of (this) experiment #{}",
                    experiment.seq
                );
            }
        }

        Ok(run_id_to_status)
    }
}

/// This function takes [SacctOutput] and expands job ids.
///
/// Ids like: `1234_[22-34]` will get expanded into `1234_22, 1234_23, ...,
/// 1234_34`.
fn flatten_job_id(jobs: Vec<SacctOutput>) -> Result<Vec<SacctOutput>> {
    let mut result = vec![];

    for job in jobs {
        for id in flatten_slurm_id(job.job_id.clone())? {
            result.push(SacctOutput {
                job_id: id,
                job_name: job.job_name.clone(),
                state: job.state.clone(),
                slurm_exit_code: job.slurm_exit_code,
                program_exit_code: job.program_exit_code,
            })
        }
    }

    Ok(result)
}

/// Takes a job id as slurm spews out, eg `1234_[56-58]`
/// and expands it into `[1234_56, 1234_57, 1234_58]`.
///
/// all ids are represented as Strings.
pub fn flatten_slurm_id(id: String) -> Result<Vec<String>> {
    let mut result = vec![];

    // Match job ids in form NUMBER_[ranges]
    // Where ranges are a comma separated list where
    // every value is either NUM or NUM-NUM
    let range = Regex::new(r"([0-9]+)_\[(..*?)\]$").with_context(ctx!("",;"",))?;

    // Match job ids in form NUMBER_NUMBER
    let solo = Regex::new(r"([0-9]+)_([0-9]+)$").with_context(ctx!("",;"",))?;

    if let Some(captures) = range.captures(&id) {
        let batch_id = &captures[1];
        let ranges = &captures[2];

        for r in ranges.split(',') {
            trace!("Captured range {r} for {id}");
            let over_separators: Vec<&str> = r.split('-').collect();

            if over_separators.len() == 2 {
                let begin: usize = over_separators[0].parse()?;
                let end: usize = over_separators[1].parse()?;

                for run_id in begin..=end {
                    result.push(format!("{}_{}", batch_id, run_id))
                }
            } else if over_separators.len() == 1 {
                let run_id: usize = over_separators[0].parse()?;
                result.push(format!("{}_{}", batch_id, run_id))
            }
        }
    }

    if let Some(captures) = solo.captures(&id) {
        let batch_id = &captures[1];
        let run_id = captures[2].parse::<usize>()?;

        result.push(format!("{}_{}", batch_id, run_id))
    }

    Ok(result)
}

#[cfg(test)]
#[path = "tests/slurm_based.rs"]
mod tests;
