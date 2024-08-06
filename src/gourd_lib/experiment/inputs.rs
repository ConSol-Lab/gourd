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
use crate::config::parameters::validate_parameters;
use crate::config::Parameter;
use crate::config::UserInput;
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
    inp: &BTreeMap<String, UserInput>,
    parameters: &Option<BTreeMap<String, Parameter>>,
    fs: &impl FileOperations,
) -> Result<InternalInputMap> {
    let mut initial = inp.clone();
    let mut out = BTreeMap::new();

    // Expand globs in arguments.
    initial = expand_argument_globs(&initial, fs)?;

    // Expand parameters.
    if let Some(params) = parameters {
        validate_parameters(params)?;
        initial = expand_parameters(initial, params)?;
    }

    // Expand file input
    for (name, user) in initial {
        match (user.file, user.glob, user.fetch) {
            (Some(f), None, None) => {
                out.insert(
                    name.clone(),
                    InternalInput {
                        input: Some(canon_path(&f, fs)?),
                        arguments: user.arguments.clone(),
                        metadata: Metadata {
                            glob_from: None,
                            is_fetched: false,
                        },
                    },
                );
            }

            (None, Some(glob), None) => {
                for glob in glob::glob(&glob)? {
                    let path = glob?;

                    if let Some(f) = path.file_stem() {
                        out.insert(
                            format!("{}_i_{f:?}", name.clone()),
                            InternalInput {
                                input: Some(canon_path(&path, fs)?),
                                arguments: user.arguments.clone(),
                                metadata: Metadata {
                                    glob_from: Some(name.clone()),
                                    is_fetched: false,
                                },
                            },
                        );
                    }
                }
            }

            (None, None, Some(fetched)) => {
                let name = format!("{name}_fetched");
                out.insert(
                    name.clone(),
                    InternalInput {
                        input: Some(canon_path(&fetched.fetch(fs)?, fs)?),
                        arguments: user.arguments.clone(),
                        metadata: Metadata {
                            glob_from: None,
                            is_fetched: true,
                        },
                    },
                );
            }
            _ => {
                bailc!(
                    "More than one file source specified or none.",;
                    "Input {name} has more than one file/glob/fetch specified
                    or none",;
                    "Split this input into one for each file/glob/fetch",
                );
            }
        }
    }

    Ok(out)
}
