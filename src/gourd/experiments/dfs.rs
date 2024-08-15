use std::collections::VecDeque;
use std::path::PathBuf;

use anyhow::Context;
use gourd_lib::bailc;
use gourd_lib::experiment::Experiment;
use gourd_lib::experiment::Run;
use gourd_lib::experiment::RunInput;
use gourd_lib::file_system::FileOperations;

use crate::experiments::run::generate_new_run;

/// A helper enum for dfs.
enum Step {
    /// We have just entered node `.0` with parents of `.1`.
    Entry(usize, Option<Vec<(usize, PathBuf)>>),

    /// We have just left node `.0`.
    Exit(usize),
}

/// A depth first search for creating the program tree.
pub(super) fn dfs(
    visitation: &mut [usize],
    start: usize,
    runs: &mut Vec<Run>,
    exp: &Experiment,
    fs: &impl FileOperations,
) -> anyhow::Result<()> {
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
                        exp.programs[node].limits,
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
