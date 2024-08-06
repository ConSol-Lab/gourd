use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::bailc;
use crate::config::maps::canon_path;
use crate::config::maps::expand_argument_globs;
use crate::config::maps::InternalInputMap;
use crate::config::parameters::expand_parameters;
use crate::config::Parameter;
use crate::config::UserInputMap;
use crate::experiment::FieldRef;
use crate::experiment::InternalInput;
use crate::experiment::Metadata;
use crate::file_system::FileOperations;

/// The input for a [`Run`], exactly as will be passed to the wrapper for
/// execution.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RunInput {
    /// The name of this input, as specified by the user.
    pub name: FieldRef,
    /// A file whose contents to be passed into the program's `stdin`
    pub file: Option<PathBuf>,
    /// Command line arguments for this binary execution.
    ///
    /// Holds the concatenation of [`UserProgram`] specified arguments and
    /// [`UserInput`] arguments.
    pub args: Vec<String>,
}

/// Convert a [`UserInput`] to a list of [`InternalInput`]s, expanding globs and
/// fetching remote resources.
pub fn expand_inputs(
    inp: &UserInputMap,
    parameters: &Option<BTreeMap<String, Parameter>>,
    fs: &impl FileOperations,
) -> Result<InternalInputMap> {
    let mut initial = inp.clone();
    let mut out = BTreeMap::new();
    // 1. expand globs in arguments
    initial = expand_argument_globs(&initial)?;
    // 2. expand parameters
    if let Some(params) = parameters {
        initial = expand_parameters(initial, params)?;
    }
    // 3. expand file input
    for (n, u) in initial {
        match (u.file, u.glob, u.fetch) {
            (Some(f), None, None) => {
                out.insert(
                    n.clone(),
                    InternalInput {
                        name: n.clone(),
                        input: Some(canon_path(&f, fs)?),
                        arguments: u.arguments.clone(),
                        metadata: Metadata { is_glob: false },
                    },
                );
            }
            (None, Some(glob), None) => {
                for glob in glob::glob(&glob)? {
                    let p = glob?;
                    if let Some(f) = p.file_stem() {
                        let name = format!("{n}_glob_{f:?}");
                        out.insert(
                            name.clone(),
                            InternalInput {
                                name,
                                input: Some(canon_path(&p, fs)?),
                                arguments: u.arguments.clone(),
                                metadata: Metadata { is_glob: true },
                            },
                        );
                    }
                }
            }
            (None, None, Some(fetched)) => {
                let name = format!("{n}_fetched");
                out.insert(
                    name.clone(),
                    InternalInput {
                        name,
                        input: Some(canon_path(&fetched.fetch(fs)?, fs)?),
                        arguments: u.arguments.clone(),
                        metadata: Metadata { is_glob: false },
                    },
                );
            }
            (None, None, None) => {
                out.insert(
                    n.clone(),
                    InternalInput {
                        name: n.clone(),
                        input: None,
                        arguments: u.arguments.clone(),
                        metadata: Metadata { is_glob: false },
                    },
                );
            }
            _ => {
                bailc!(
                    "More than one file source specified.",;
                    "Input {n} has more than one file/glob/fetch specified",;
                    "Split this input into one for each file/glob/fetch",
                );
            }
        }
    }

    Ok(out)
}

/// Convert an iterator of tuples into a BTreeMap
pub fn iter_map<X: Ord, Y>(i: std::vec::IntoIter<(X, Y)>) -> BTreeMap<X, Y> {
    let mut map = BTreeMap::new();

    for (x, y) in i {
        map.insert(x, y);
    }

    map
}
