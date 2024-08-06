use std::collections::BTreeMap;

use anyhow::Context;
use anyhow::Result;

use crate::bailc;
use crate::config::maps::canon_path;
use crate::config::Config;
use crate::config::UserProgram;
use crate::experiment::InternalProgram;
use crate::file_system::FileOperations;

/// Convert a [`UserProgram`] to a list of [`InternalProgram`]s,
/// expanding globs and fetching remote resources.
pub fn expand_programs(
    prog: &BTreeMap<String, UserProgram>,
    conf: &Config,
    fs: &impl FileOperations,
) -> Result<BTreeMap<String, InternalProgram>> {
    let mut out = BTreeMap::new();

    for (name, user) in prog {
        let file = canon_path(
            &match (&user.binary, &user.fetch) {
                (Some(f), None) => f.clone(),
                (None, Some(fetched)) => fetched.fetch(fs)?,
                _ => {
                    bailc!(
                        "Wrong number of file sources specified.",;
                        "Program {name} does not have one binary/fetch
                        specified",;
                        "Specify exactly one binary source per program.",
                    );
                }
            },
            fs,
        )?;
        let limits = user
            .resource_limits
            .unwrap_or(conf.resource_limits.unwrap_or_default());

        for child in &user.next {
            if !prog.contains_key(child) {
                bailc!(
                        "Incorrect program dependency: {}", child;
                        "Program {child} runs on {name}, but there's no program
                        called {child}!",;
                        "Please make sure all programs exist and spelling
                        is correct",);
            }
        }

        out.insert(
            name.clone(),
            InternalProgram {
                binary: file,
                afterscript: user.afterscript.clone(),
                limits,
                arguments: user.arguments.clone(),
                next: user.next.clone(),
            },
        );
    }

    Ok(out)
}
