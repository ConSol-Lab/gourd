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
) -> Result<Vec<InternalProgram>> {
    let mut out = Vec::new();
    let mut mapper = BTreeMap::new();

    for (name, user) in prog {
        let file = canon_path(
            &match (&user.binary, &user.fetch, &user.git) {
                (Some(f), None, None) => f.clone(),
                (None, Some(fetched), None) => fetched.fetch(fs)?,
                (None, None, Some(git)) => git.fetch(fs)?,

                _ => {
                    bailc!(
                        "Wrong number of file sources specified.",;
                        "Program {name} does not have 1 binary/fetch specified",;
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
                    "Program {child} runs on {name}, but there's no program called {child}!",;
                    "Please make sure all programs exist and spelling is correct",
                );
            }
        }

        let env_map = match &user.env {
            Some(user_env_map) => {user_env_map.clone()},
            None => BTreeMap::new(),
        };

        mapper.insert(name, out.len());
        out.push(InternalProgram {
            name: name.to_string(),
            binary: file,
            afterscript: user
                .afterscript
                .clone()
                .map(|p| canon_path(&p, fs))
                .transpose()?,
            limits,
            arguments: user.arguments.clone(),
            env: env_map,
            next: Vec::new(),
        });
    }

    for out_prog in out.iter_mut() {
        for next_norm in &prog[&out_prog.name].next {
            out_prog.next.push(mapper[next_norm]);
        }
    }

    Ok(out)
}
