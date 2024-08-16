use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use chrono::DateTime;
use chrono::Local;
use gourd_lib::bailc;
use gourd_lib::config::slurm::SlurmConfig;
use gourd_lib::constants::SHORTEN_STATUS_CUTOFF;
use gourd_lib::constants::SLURM_VERSIONS;
use gourd_lib::constants::TERTIARY_STYLE;
use gourd_lib::ctx;
use gourd_lib::experiment::Experiment;
use log::debug;
use log::info;
use log::trace;
use regex_lite::Regex;

use super::handler::parse_optional_args;
use super::SacctOutput;
use crate::chunks::Chunk;
use crate::chunks::Chunkable;
use crate::slurm::SlurmInteractor;

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

/// Get a limit from the `sacctmgr` command.
fn sacctmgr_limit(field: &str) -> Result<String> {
    let mut cmd = Command::new("sacctmgr");
    cmd.arg("show")
        .arg("user")
        .arg("withassoc")
        .arg("where")
        .arg(format!("name={}", std::env::var("USER")?))
        .arg(format!("format={}", field))
        .arg("--parsable2")
        .arg("--noheader");

    let out = cmd.output()?;
    debug!("Running {:?} gave {:?}", cmd, &out);

    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Get a limit from the `scontrol` command.
fn scontrol_limit(field: &str) -> Result<String> {
    let s_control_out = Command::new("scontrol")
        .arg("show")
        .arg("config")
        .output()?
        .stdout;
    let s_control_str = String::from_utf8_lossy(&s_control_out);
    Ok(s_control_str
        .split('\n')
        .filter(|x| x.contains(field))
        .collect::<Vec<&str>>()[0]
        .split('=')
        .collect::<Vec<&str>>()[1]
        .trim()
        .to_string())
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
        match scontrol_limit("MaxArraySize")?.parse() {
            Ok(x) => Ok(x),
            Err(e) => {
                debug!("Could not parse max array size from slurm: {e}");
                Ok(usize::MAX)
            }
        }
    }

    fn max_submit(&self) -> Result<usize> {
        let out = Command::new("sacctmgr")
            .arg("show")
            .arg("qos")
            .arg("format=MaxSubmitPU")
            .arg("--parsable2")
            .arg("--noheader")
            .output()?;

        if let Some(max_submit) = String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .next()
        {
            Ok(max_submit.parse()?)
        } else {
            match sacctmgr_limit("MaxSubmit")?.parse() {
                Ok(x) => Ok(x),
                Err(e) => {
                    debug!("Could not parse max submissions from slurm: {e}");
                    Ok(usize::MAX)
                }
            }
        }
    }

    fn max_jobs(&self) -> Result<usize> {
        match sacctmgr_limit("MaxJobs")?.parse() {
            Ok(x) => Ok(x),
            Err(e) => {
                debug!("Could not parse max job count from slurm: {e}");
                Ok(usize::MAX)
            }
        }
    }

    fn max_cpu(&self) -> Result<usize> {
        match sacctmgr_limit("MaxCPUs")?.parse() {
            Ok(x) => Ok(x),
            Err(e) => {
                debug!("Could not parse max cpus allowed from slurm: {e}");
                Ok(usize::MAX)
            }
        }
    }

    fn max_memory(&self) -> Result<usize> {
        todo!()
    }

    fn max_time(&self) -> Result<Duration> {
        let time = &sacctmgr_limit("MaxWallDurationPerJob")?;
        // According to slurm docs:
        // "Maximum wall clock time each job is able to use in this association.
        // This is overridden if set directly on a user. Default is the cluster's limit.
        // <max wall> format is
        // * <min> or
        // * <min>:<sec> or
        // * <hr>:<min>:<sec> or
        // * <days>-<hr>:<min>:<sec> or
        // * <days>-<hr>.
        // The value is recorded in minutes with rounding as needed."

        let a = Regex::new(r"(\d+)-(\d+):(\d+):(\d+)")?;
        let b = Regex::new(r"(\d+)-(\d+)")?;
        let c = Regex::new(r"(\d+):(\d+):(\d+)")?;
        let d = Regex::new(r"(\d+):(\d+)")?;
        let e = Regex::new(r"(\d+)")?;

        let err = "regex error in slurm interactor";
        match time {
            x if a.is_match(x) => {
                if let Some(caps) = a.captures(x) {
                    let days = caps.get(1).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    let hours = caps.get(2).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    let minutes = caps.get(3).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    let seconds = caps.get(4).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    Ok(Duration::from_secs(
                        days * 24 * 60 * 60 + hours * 60 * 60 + minutes * 60 + seconds,
                    ))
                } else {
                    unreachable!("No captures from matching regex (??)")
                }
            }
            y if b.is_match(y) => {
                if let Some(caps) = b.captures(y) {
                    let days = caps.get(1).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    let hours = caps.get(2).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    Ok(Duration::from_secs(days * 24 * 60 * 60 + hours * 60 * 60))
                } else {
                    unreachable!("No captures from matching regex (??)")
                }
            }
            z if c.is_match(z) => {
                if let Some(caps) = c.captures(z) {
                    let hours = caps.get(1).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    let minutes = caps.get(2).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    let seconds = caps.get(3).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    Ok(Duration::from_secs(
                        hours * 60 * 60 + minutes * 60 + seconds,
                    ))
                } else {
                    unreachable!("No captures from matching regex (??)")
                }
            }
            w if d.is_match(w) => {
                if let Some(caps) = d.captures(w) {
                    let hours = caps.get(1).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    let minutes = caps.get(2).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    Ok(Duration::from_secs(hours * 60 * 60 + minutes * 60))
                } else {
                    unreachable!("No captures from matching regex (??)")
                }
            }
            _ => {
                if let Some(caps) = e.captures(time) {
                    let minutes = caps.get(1).ok_or(anyhow!(err))?.as_str().parse::<u64>()?;
                    Ok(Duration::from_secs(minutes * 60))
                } else {
                    Ok(Duration::MAX)
                }
            }
        }
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

        trace!("sbatch child: {:?}", &cmd);

        let proc = cmd.wait_with_output().context("")?;

        debug!("sbatch exited with code {:?}", proc.status);
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

    fn get_accounting_data(&self, since: &DateTime<Local>) -> Result<Vec<SacctOutput>> {
        let mut sacct_cmd = Command::new("sacct");
        sacct_cmd
            .arg("-p")
            .arg("--allocations")
            .arg("--starttime")
            .arg(since.format("%Y-%m-%d %H:%M:%S").to_string()) // YYYY-MM-DD[THH:MM[:SS]] from slurm docs
            .arg("--endtime=now")
            .arg("--format=jobid,jobname,state,exitcode");

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

    fn scheduled_jobs(&self) -> Result<Vec<String>> {
        let sacct = Command::new("squeue")
            .arg("--me")
            .arg("--array")
            .arg("-h") // hide the table header
            .arg("--Format=jobid,arraytaskid")
            .arg("--state=PD,R,CG")
            .output()
            .with_context(ctx!(
              "Could not get scheduled jobs", ;
              "Make sure that the `squeue` program is accessible",
            ))?;

        Ok(String::from_utf8_lossy(&sacct.stdout)
            .lines()
            .map(|line| {
                let mut parts = line.split_whitespace();
                match (parts.next(), parts.next()) {
                    (Some(job), Some(task)) => format!("{}_{}", job.trim(), task.trim()),
                    _ => unreachable!("Error in parsing squeue output. This should not happen."),
                }
            })
            .collect())
    }

    fn scheduled_count(&self) -> Result<usize> {
        let sacct = Command::new("squeue")
            .arg("--me")
            .arg("--array")
            .arg("-h") // hide the table header
            .arg("--Format=jobid")
            .arg("--state=PD,R,CG")
            .output()
            .with_context(ctx!(
              "Could not get scheduled jobs", ;
              "Make sure that the `squeue` program is accessible",
            ))?;

        Ok(String::from_utf8_lossy(&sacct.stdout).lines().count())
    }

    fn cancel_jobs(&self, batch_ids: Vec<String>) -> Result<()> {
        if batch_ids.len() < SHORTEN_STATUS_CUTOFF {
            info!(
                "Cancelling runs {TERTIARY_STYLE}[{}]{TERTIARY_STYLE:#}",
                batch_ids.join(", ")
            );
        } else {
            info!("Cancelling {} runs", batch_ids.len());
        }

        let mut cancel = Command::new("scancel");
        cancel.args(&batch_ids);

        debug!("Running cancel: {:?}", cancel);

        let output = cancel.output().with_context(ctx!(
          "Failed to cancel runs",;
          "Make sure that the `scancel` program is accessible",
        ))?;

        if !output.status.success() {
            bailc!("Failed to cancel runs", ;
                "scancel printed: {}", String::from_utf8(output.stderr).unwrap();
                "",
            );
        } else {
            info!("{} runs cancelled", batch_ids.len());
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/interactor.rs"]
mod tests;
