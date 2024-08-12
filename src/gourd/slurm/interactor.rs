use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use gourd_lib::bailc;
use gourd_lib::config::SlurmConfig;
use gourd_lib::constants::SLURM_VERSIONS;
use gourd_lib::ctx;
use gourd_lib::experiment::Experiment;
use log::debug;
use log::trace;

use super::handler::parse_optional_args;
use super::SacctOutput;
use crate::chunks::Chunk;
use crate::chunks::Chunkable;
use crate::slurm::SlurmInteractor;
use crate::status::slurm_based::flatten_slurm_id;

/// Creates a Slurm duration string.
///
/// Converts a standard `std::time::Duration` to a Slurm duration in one of
/// the following formats: {ss, mm:ss, hh:mm:ss, d-hh:mm:ss}
pub fn format_slurm_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let secs_rem = secs % 60;

    if secs == secs_rem {
        return format!("{:0>2}", secs);
    }

    let mins = secs / 60;
    let mins_rem = mins % 60;

    if mins == mins_rem {
        return format!("{:0>2}:{:0>2}", mins, secs_rem);
    }

    let hours = mins / 60;
    let hours_rem = hours % 24;

    if hours == hours_rem {
        return format!("{:0>2}:{:0>2}:{:0>2}", hours, mins_rem, secs_rem);
    }

    let days = hours / 24;

    format!(
        "{}-{:0>2}:{:0>2}:{:0>2}",
        days, hours_rem, mins_rem, secs_rem
    )
}

/// An implementation of the SlurmInteractor trait for interacting with SLURM
/// via the CLI.
#[derive(Debug)]
pub struct SlurmCli {
    /// Supported versions by this instance of the CLI interactor
    pub versions: Vec<[u64; 2]>,
}

impl Default for SlurmCli {
    fn default() -> Self {
        Self {
            versions: SLURM_VERSIONS.to_vec(),
        }
    }
}

/// These are using functions specific to CLI version 21.8.x
///
/// we don't know if other versions are supported.
#[cfg(not(tarpaulin_include))]
impl SlurmInteractor for SlurmCli {
    fn get_version(&self) -> Result<[u64; 2]> {
        let s_info_out = Command::new("sinfo").arg("--version").output()?;
        let version = String::from_utf8_lossy(&s_info_out.stdout)
            .to_string()
            .split_whitespace()
            .collect::<Vec<&str>>()[1]
            .split(|c: char| !c.is_numeric())
            .collect::<Vec<&str>>()
            .iter()
            .map(|x| x.parse::<u64>().unwrap())
            .collect::<Vec<u64>>();
        let mut buf = [0; 2];

        buf[0] = *version.first().ok_or(anyhow!("Invalid version received"))?;
        buf[1] = *version.get(1).ok_or(anyhow!("Invalid version received"))?;

        Ok(buf)
    }

    fn get_partitions(&self) -> Result<Vec<Vec<String>>> {
        let s_info_out = Command::new("sinfo").arg("-o").arg("%P %a").output()?;
        let partitions = String::from_utf8_lossy(&s_info_out.stdout)
            .split('\n')
            .map(|x| x.to_string())
            .map(|y| {
                y.split_whitespace()
                    .collect::<Vec<&str>>()
                    .iter()
                    .map(|z| z.to_string())
                    .collect::<Vec<String>>()
            })
            .collect::<Vec<Vec<String>>>();
        Ok(partitions)
    }

    fn max_array_size(&self) -> Result<usize> {
        let s_control_out = Command::new("scontrol")
            .arg("show")
            .arg("config")
            .output()?
            .stdout;
        let s_control_str = String::from_utf8_lossy(&s_control_out);
        let max_array_size = s_control_str
            .split('\n')
            .filter(|x| x.contains("MaxArraySize"))
            .collect::<Vec<&str>>()[0]
            .split('=')
            .collect::<Vec<&str>>()[1]
            .trim()
            .parse::<usize>()
            .context("Could not parse max array size")?;
        Ok(max_array_size)
    }

    fn schedule_chunk(
        &self,
        slurm_config: &SlurmConfig,
        chunk: &Chunk,
        experiment: &mut Experiment,
        exp_path: &Path,
    ) -> Result<()> {
        let resource_limits = chunk.limits();
        let chunk_index = experiment.register_runs(&chunk.runs);

        let optional_args = parse_optional_args(slurm_config);

        // `%A` gets replaced with array *job* id, `%a` with the array *task* id
        // this is read in `src/gourd/status/slurm_files.rs` to get the output.
        let slurm_out = experiment
            .slurm_out("%A_%a")
            .ok_or(anyhow!("Slurm config not found (unreachable)"))?;
        let slurm_err = experiment
            .slurm_err("%A_%a")
            .ok_or(anyhow!("Slurm config not found (unreachable)"))?;

        let contents = format!(
            "#!/bin/bash
#SBATCH --job-name=\"{}\"
#SBATCH --array=\"{}-{}\"
#SBATCH --ntasks=1
#SBATCH --partition=\"{}\"
#SBATCH --time=\"{}\"
#SBATCH --cpus-per-task=\"{}\"
#SBATCH --mem-per-cpu=\"{}\"
#SBATCH --account=\"{}\"
#SBATCH --output={:?}
#SBATCH --error={:?}
{}
set -x

{} {} {} $SLURM_ARRAY_TASK_ID
",
            slurm_config.experiment_name,
            0,
            &chunk.runs.len() - 1,
            slurm_config.partition,
            format_slurm_duration(resource_limits.time_limit),
            resource_limits.cpus,
            resource_limits.mem_per_cpu,
            slurm_config.account,
            slurm_out,
            slurm_err,
            optional_args,
            experiment.wrapper,
            exp_path.display(),
            chunk_index
        );

        debug!("Sbatch file: {}", contents);

        let mut cmd = Command::new("sbatch")
            .arg("--parsable")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .with_context(ctx!(
              "Failed to submit batch job to SLURM", ;
              "Ensure that you have permissions to submit jobs to the cluster",
            ))?;

        cmd.stdin
            .as_mut()
            .ok_or(anyhow!("Could not connect to sbatch"))
            .context("")?
            .write_all(contents.as_bytes())
            .with_context(ctx!(
                "Failed to submit batch job to SLURM", ;
                "Tried submitting this script {contents}",
            ))?;

        let proc = cmd.wait_with_output().context("")?;

        trace!("sbatch exited with code {:?}", proc.status);
        trace!("sbatch stdout:\n{:?}", proc.stdout);
        trace!("sbatch stderr:\n{:?}", proc.stderr);

        if !proc.status.success() {
            bailc!("Sbatch failed to run", ;
                "Sbatch printed: {}", String::from_utf8(proc.stderr).unwrap();
                "Please ensure that you are running on slurm",
            );
        }

        let batch_id = String::from_utf8(proc.stdout)
            .with_context(ctx!(
              "Could not decode sbatch output", ; "",
            ))?
            .trim()
            .to_string();

        trace!("This chunk was scheduled with id: {batch_id}");
        experiment.mark_chunk_scheduled(chunk, batch_id);

        Ok(())
    }

    fn is_version_supported(&self, v: [u64; 2]) -> bool {
        self.versions.contains(&v)
    }

    fn get_supported_versions(&self) -> String {
        self.versions
            .iter()
            .map(|x| format!("{}.{}", x[0], x[1]))
            .collect::<Vec<String>>()
            .join(", ")
    }

    fn get_accounting_data(&self, job_ids: Vec<String>) -> Result<Vec<SacctOutput>> {
        let mut sacct_cmd = Command::new("sacct");
        sacct_cmd
            .arg("-p")
            .arg("--format=jobid,jobname,state,exitcode")
            .arg(format!("--jobs={}", job_ids.join(",")));

        trace!("Gathering slurm status with: {sacct_cmd:?}");

        let sacct = sacct_cmd.output().with_context(ctx!(
          "Could not get accounting data", ;
          "Make sure that the `sacct` program is accessible",
        ))?;

        let mut result = Vec::new();

        for job in String::from_utf8_lossy(&sacct.stdout)
            .trim()
            .split('\n')
            .skip(1)
        {
            let fields = job.split('|').collect::<Vec<&str>>();
            let exit_codes = fields[3].split(':').collect::<Vec<&str>>();

            result.push(SacctOutput {
                job_id: fields[0].to_string(),
                job_name: fields[1].to_string(),
                state: fields[2].to_string(),
                slurm_exit_code: exit_codes[0].parse().unwrap_or(0),
                program_exit_code: exit_codes[1].parse().unwrap_or(0),
            });
        }

        Ok(result)
    }

    fn get_scheduled_jobs(&self) -> Result<Vec<String>> {
        let sacct = Command::new("sacct")
            .arg("-p")
            .arg("--format=jobid,state")
            .output()
            .with_context(ctx!(
              "Could not get scheduled jobs", ;
              "Make sure that the `sacct` program is accessible",
            ))?;

        let mut result: BTreeSet<String> = BTreeSet::new();

        for job in String::from_utf8_lossy(&sacct.stdout)
            .trim()
            .split('\n')
            .skip(1)
        {
            let fields = job.split('|').collect::<Vec<&str>>();
            let set_of_completed_states = BTreeSet::from([
                "PD", "PENDING", "R", "RUNNING", "RQ", "REQUEUED", "RS", "RESIZING",
            ]);

            if set_of_completed_states.contains(fields[1]) {
                debug!("Found {} as possible id", fields[0]);
                flatten_slurm_id(fields[0].to_string())?
                    .iter()
                    .for_each(|x| {
                        result.insert(x.to_string());
                    });
            }
        }

        Ok(result.into_iter().collect::<Vec<String>>())
    }

    fn cancel_jobs(&self, batch_ids: Vec<String>) -> Result<()> {
        for batch_id in batch_ids {
            debug!("Cancelling batch with id: \"scancel {}\"", batch_id);

            if Command::new("scancel")
                .arg(batch_id.clone())
                .status()
                .is_err()
            {
                bailc!(
                    "Failed to cancel the job with batch id {batch_id}", ;
                    "", ;
                    "",
                )
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/interactor.rs"]
mod tests;
