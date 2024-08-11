use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use chrono::DateTime;
use chrono::Local;
use gourd_lib::bailc;
use gourd_lib::config::Config;
use gourd_lib::config::ResourceLimits;
use gourd_lib::ctx;
use gourd_lib::experiment::inputs::expand_inputs;
use gourd_lib::experiment::labels::Labels;
use gourd_lib::experiment::programs::expand_programs;
use gourd_lib::experiment::Environment;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::FieldRef;
use gourd_lib::experiment::Run;
use gourd_lib::experiment::RunInput;
use gourd_lib::file_system::FileOperations;

/// Extension trait for the shared `Experiment` struct.
pub trait ExperimentExt {
    /// Initialize a new experiment from a `config`.
    ///
    /// Creates a new experiment by matching all algorithms to all inputs.
    /// The experiment is created in the provided `env` and with `time` as the
    /// timestamp.
    fn from_config(
        conf: &Config,
        time: DateTime<Local>,
        env: Environment,
        fs: &impl FileOperations,
    ) -> Result<Self>
    where
        Self: Sized;

    /// Get the filename of the newest experiment.
    fn latest_id_from_folder(folder: &Path) -> Result<Option<usize>>;

    /// Provided a folder gets the most recent experiment.
    fn latest_experiment_from_folder(folder: &Path, fs: &impl FileOperations)
        -> Result<Experiment>;

    /// Provided a folder gets the specified experiment.
    fn experiment_from_folder(
        seq: usize,
        folder: &Path,
        fs: &impl FileOperations,
    ) -> Result<Experiment>;
}

impl ExperimentExt for Experiment {
    fn from_config(
        conf: &Config,
        time: DateTime<Local>,
        env: Environment,
        fs: &impl FileOperations,
    ) -> Result<Self> {
        let seq = Self::latest_id_from_folder(&conf.experiments_folder)
            .unwrap_or(Some(0))
            .unwrap_or(0)
            + 1;

        // First we will explode all programs from the initial set to their final set.
        let expanded_programs = expand_programs(&conf.programs, conf, fs)?;

        // Now we will expand all inputs in a similar manner.
        let expanded_inputs = expand_inputs(&conf.inputs, &conf.parameters, fs)?;

        let mut experiment = Self {
            seq,
            creation_time: time,
            home: conf.experiments_folder.clone(),
            wrapper: conf.wrapper.clone(),

            inputs: expanded_inputs.clone(),
            programs: expanded_programs,

            output_folder: conf.output_path.clone(),
            metrics_folder: conf.metrics_path.clone(),
            afterscript_output_folder: Default::default(), //todo

            env,
            resource_limits: conf.resource_limits,
            labels: Labels {
                map: conf.labels.clone().unwrap_or_default(),
                warn_on_label_overlap: conf.warn_on_label_overlap,
            },

            slurm: conf.slurm.clone(),

            runs: Vec::new(),
        };

        // Collapse the programs into an ordered structure for faster processing.
        let mut in_degrees = vec![0usize; experiment.programs.len()];

        for prog in &experiment.programs {
            for next_prog in &prog.next {
                in_degrees[*next_prog] += 1;
            }
        }

        let mut visitation = vec![0usize; experiment.programs.len()];
        let mut runs = Vec::new();

        for (prog, degree) in in_degrees.iter().enumerate() {
            if *degree == 0 {
                dfs(&mut visitation, prog, &mut runs, &experiment, fs)?;
            }
        }

        for (prog, visit) in visitation.iter().enumerate() {
            if *visit != 1 {
                bailc!(
                    "A cycle was found in the program dependencies.",;
                    "The `next` field in the program definitions created a circular dependency",;
                    "Fix the dependencies for {:?}",experiment.programs[prog].name
                );
            }
        }

        experiment.runs = runs;

        Ok(experiment)
    }

    fn latest_id_from_folder(folder: &Path) -> Result<Option<usize>> {
        let mut highest = None;

        // More lenient handling of other files in the experiment directory,
        // as long as they are not *.lock files.
        // This is in line with some of our examples, which have been adjusted
        // to also put output and metrics in the experiment dir for cleanness.
        let file_names = fs::read_dir(folder).with_context(ctx!(
          "Could not access the experiments directory {folder:?}", ;
          "Run some experiments first or ensure that you have sufficient permissions to read it",
        ))?.flatten()
            // get only regular files
            .filter(|f| f.file_type().is_ok_and(|fty| fty.is_file()))
            // get file names
            .filter_map(|f| f.file_name().into_string().ok())
            // get only .lock files
            .filter(|name| name.ends_with(".lock"));

        for file_name in file_names {
            let seq_of_file = file_name
                .trim_end_matches(".lock")
                .parse()
                .with_context(ctx!(
                  "Invalid name of experiment file {:?}", file_name;
                  "Do not manually modify .lock files in the experiment directory",
                ))?;

            if highest.is_none() {
                highest = Some(seq_of_file);
            } else if let Some(num) = highest {
                if seq_of_file > num {
                    highest = Some(seq_of_file);
                }
            }
        }

        Ok(highest)
    }

    fn latest_experiment_from_folder(
        folder: &Path,
        fs: &impl FileOperations,
    ) -> Result<Experiment> {
        if let Some(id) = Self::latest_id_from_folder(folder)? {
            Self::experiment_from_folder(id, folder, fs)
        } else {
            bailc!(
                "There are no experiments, try running some first", ;
                "", ;
                "",
            );
        }
    }

    fn experiment_from_folder(
        seq: usize,
        folder: &Path,
        fs: &impl FileOperations,
    ) -> Result<Experiment> {
        fs.try_read_toml(&folder.join(format!("{}.lock", seq)))
    }
}

/// A helper enum for dfs.
enum Step {
    /// We have just entered node `.0` with parents of `.1`.
    Entry(usize, Option<Vec<(usize, PathBuf)>>),

    /// We have just left node `.0`.
    Exit(usize),
}

/// A depth first search for creating the prog tree.
fn dfs(
    visitation: &mut [usize],
    start: usize,
    runs: &mut Vec<Run>,
    exp: &Experiment,
    fs: &impl FileOperations,
) -> Result<()> {
    // Since the run amount can be in the millions I don't want to rely on tail
    // recursion, and we will just use unrolled dfs.
    let mut next: VecDeque<Step> = VecDeque::new();
    next.push_back(Step::Entry(start, None));

    while let Some(step) = next.pop_front() {
        if let Step::Entry(node, parent) = step {
            // We jumped backward in the search tree.
            if visitation[node] == 2 {
                bailc!(
                    "A cycle was found in the program dependencies!",;
                    "The `next` field in some program definition created a circular dependency",;
                    "This incident will be reported",
                );
            }

            // We've seen this node in a different part of the search tree.
            if visitation[node] == 1 {
                continue;
            }

            // We've never seen this node before.

            visitation[node] = 2;

            let mut children = Vec::new();

            if parent.is_none() {
                for (input_name, input) in &exp.inputs {
                    let child = generate_new_run(
                        runs.len(),
                        node,
                        RunInput {
                            file: input.input.clone(),
                            arguments: input.arguments.clone(),
                        },
                        Some(input_name.clone()),
                        exp.programs[node].limits, // todo: check what node is
                        None,
                        exp,
                        fs,
                    )?;

                    children.push((runs.len(), child.output_path.clone()));
                    runs.push(child);
                }
            } else if let Some(pchildren) = parent {
                for pchild in pchildren {
                    let child = generate_new_run(
                        runs.len(),
                        node,
                        RunInput {
                            file: Some(pchild.1),
                            arguments: runs[pchild.0].input.arguments.clone(),
                        },
                        None,
                        runs[pchild.0].limits,
                        Some(pchild.0),
                        exp,
                        fs,
                    )?;

                    children.push((runs.len(), child.output_path.clone()));
                    runs.push(child);
                }
            }

            for child in &exp.programs[node].next {
                next.push_back(Step::Entry(*child, Some(children.clone())));
            }

            next.push_back(Step::Exit(node));
        } else if let Step::Exit(node) = step {
            visitation[node] = 1;
        }
    }

    Ok(())
}

/// This function will generate a new run.
///
/// This should be used by all code paths adding runs to the experiment.
/// This does *not* set the parent and child.
#[allow(clippy::too_many_arguments)]
pub fn generate_new_run(
    run_id: usize,
    program: usize,
    run_input: RunInput,
    input: Option<FieldRef>,
    limits: ResourceLimits,
    parent: Option<usize>,
    experiment: &Experiment,
    fs: &impl FileOperations,
) -> Result<Run> {
    Ok(Run {
        program,
        input: run_input,
        err_path: fs.truncate_and_canonicalize(
            &experiment
                .output_folder
                .join(format!("{}/{}/{}/stderr", experiment.seq, program, run_id)),
        )?,
        metrics_path: fs.truncate_and_canonicalize(
            &experiment
                .metrics_folder
                .join(format!("{}/{}/{}/metrics", experiment.seq, program, run_id)),
        )?,
        output_path: fs.truncate_and_canonicalize(
            &experiment
                .output_folder
                .join(format!("{}/{}/{}/stdout", experiment.seq, program, run_id)),
        )?,
        work_dir: fs.truncate_and_canonicalize_folder(
            &experiment
                .output_folder
                .join(format!("{}/{}/{}/", experiment.seq, program, run_id)),
        )?,
        afterscript_output_path: match experiment.programs[program].afterscript.as_ref() {
            None => None,
            Some(_) => Some(
                fs.truncate_and_canonicalize_folder(
                    &experiment
                        .output_folder
                        .join(format!("{}/{}/{}/", experiment.seq, program, run_id)),
                )?,
            ),
        },
        limits,
        slurm_id: None,
        rerun: None,
        generated_from_input: input,
        parent,
    })
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
