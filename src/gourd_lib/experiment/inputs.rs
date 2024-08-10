use std::collections::BTreeMap;

use anyhow::ensure;
use anyhow::Context;
use anyhow::Result;

use crate::bailc;
use crate::config::maps::canon_path;
use crate::config::maps::expand_argument_globs;
use crate::config::maps::InternalInputMap;
use crate::config::parameters::expand_parameters;
use crate::config::parameters::validate_parameters;
use crate::config::Parameter;
use crate::config::UserInput;
use crate::experiment::InternalInput;
use crate::experiment::Metadata;
use crate::file_system::FileOperations;

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
                            format!("{}_i_{}", &name, f.to_str().unwrap_or("")),
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
                    name,
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
            (None, None, None) => {
                ensure!(
                    !user.arguments.is_empty(),
                    "Empty inputs are not allowed! \
                ({name:?} has no file, glob, fetch or arguments specified)"
                );
                out.insert(
                    name.clone(),
                    InternalInput {
                        input: None,
                        arguments: user.arguments.clone(),
                        metadata: Metadata {
                            glob_from: None,
                            is_fetched: false,
                        },
                    },
                );
            }
            _ => {
                bailc!(
                    "Wrong number of file sources specified.",;
                    "Input {name:?} has more than one file/glob/fetch specified",;
                    "Split this input into one for each file/glob/fetch",
                );
            }
        }
    }

    Ok(out)
}
