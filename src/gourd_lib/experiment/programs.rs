use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Debug;

use anyhow::Context;
use anyhow::Result;

use crate::bailc;
use crate::config::maps::canon_path;
use crate::config::maps::InternalProgramMap;
use crate::config::Config;
use crate::config::UserProgramMap;
use crate::experiment::InternalProgram;
use crate::file_system::FileOperations;

/// Convert a [`UserProgram`] to a list of [`InternalProgram`]s,
/// expanding globs and fetching remote resources.
pub fn expand_programs(
    prog: &UserProgramMap,
    conf: &Config,
    fs: &impl FileOperations,
) -> Result<InternalProgramMap> {
    let mut out = BTreeMap::new();

    for (n, u) in prog.clone() {
        let file = canon_path(
            &match (u.binary, u.fetch) {
                (Some(f), None) => f,
                (None, Some(fetched)) => fetched.fetch(fs)?,
                _ => {
                    bailc!(
                        "Wrong number of file sources specified.",;
                        "Program {n} does not have one binary/fetch specified",;
                        "Specify exactly one binary source per program.",
                    );
                }
            },
            fs,
        )?;
        let limits = if let Some(lim) = u.resource_limits {
            lim
        } else {
            conf.resource_limits.unwrap_or_default()
        };
        if let Some(dependencies) = u.runs_after {
            for parent in dependencies {
                if prog.contains_key(&parent) {
                    out.insert(
                        n.clone(), // this must match below \/
                        InternalProgram {
                            name: n.clone(), // this must match above /\
                            binary: file.clone(),
                            afterscript: u.afterscript.clone(),
                            limits,
                            arguments: u.arguments.clone(),
                            runs_after: Some(parent.clone()),
                        },
                    );
                } else {
                    bailc!(
                        "Incorrect program dependency: {}", parent;
                        "Program {n} runs on {parent}, but there's no program called {parent}!",;
                        "Please make sure all programs exist and spelling is correct",
                    );
                }
            }
        } else {
            out.insert(
                n.clone(), // this must match below \/
                InternalProgram {
                    name: n.clone(), // this must match above /\
                    binary: file.clone(),
                    afterscript: u.afterscript.clone(),
                    limits,
                    arguments: u.arguments.clone(),
                    runs_after: None,
                },
            );
        }
    }

    Ok(out)
}

/// A child node, holds a reference to its parents
pub trait Child<X> {
    /// Get the parents of this node
    fn parents(&self) -> Vec<X>;
}

impl Child<String> for InternalProgram {
    fn parents(&self) -> Vec<String> {
        // since (for now) we only support linear dependency of runs, there's only ever
        // 1 or 0 parents per node. If this changes for any reason, this needs
        // to be updated to match
        self.runs_after.clone().map(|x| vec![x]).unwrap_or_default()
    }
}

/// A topological ordering of nodes.
///
/// A node is a pair (tuple `(X, Y)`) where `X: impl Eq` is the node identifier
/// (must be unique within the iterator!) and `Y: impl Child<X>` can be used to
/// find the identifiers of the parents
pub fn topological_ordering<X: Eq + Ord + Clone + Debug, Y: Child<X> + Clone + Debug>(
    graph: &BTreeMap<X, Y>,
) -> Result<Vec<(X, Y)>> {
    let mut stack = vec![];
    let mut visited = BTreeSet::new();

    let mut temp_mark = BTreeSet::new();

    for node in graph.keys() {
        if !visited.contains(node) {
            visit(node, graph, &mut visited, &mut temp_mark, &mut stack)?;
        }
    }

    // the stack is built with the last finished node on top
    stack.reverse();

    Ok(stack)
}

/// Helper function to visit nodes
fn visit<X: Eq + Ord + Clone + Debug, Y: Child<X> + Clone + Debug>(
    node: &X,
    graph: &BTreeMap<X, Y>,
    visited: &mut BTreeSet<X>,
    temp_mark: &mut BTreeSet<X>,
    stack: &mut Vec<(X, Y)>,
) -> Result<()> {
    if temp_mark.contains(node) {
        bailc!(
            "There is a circular dependency between programs!",;
            "{node:?} shouldn't be part of {temp_mark:?}",;
            "Program dependencies should be linear. Resolve this issue and try again.",
        );
    }

    if !visited.contains(node) {
        temp_mark.insert(node.clone());

        if let Some(children) = graph.get(node) {
            for parent in children.parents() {
                visit(&parent, graph, visited, temp_mark, stack)?;
            }
        }

        temp_mark.remove(node);
        visited.insert(node.clone());

        if let Some(entry) = graph.get(node) {
            stack.push((node.clone(), entry.clone()));
        }
    }

    Ok(())
}
