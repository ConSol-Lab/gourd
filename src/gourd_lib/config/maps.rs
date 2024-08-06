use std::collections::BTreeMap;
use std::collections::HashSet;
use std::env::current_dir;
use std::mem::swap;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use glob::glob;

use super::UserInput;
use crate::constants::GLOB_ESCAPE;
use crate::constants::INTERNAL_GLOB;
use crate::constants::INTERNAL_PREFIX;
use crate::experiment::InternalInput;
use crate::experiment::InternalProgram;
use crate::file_system::FileOperations;

/// Storing [`InternalInput`]s in a [`BTreeMap`] with their user-given names as
/// keys
pub type InternalInputMap = BTreeMap<String, InternalInput>;

/// Storing [`InternalProgram`]s in a [`BTreeMap`] with their user-given names
/// as keys
pub type InternalProgramMap = BTreeMap<String, InternalProgram>;

/// This will take a path and canonicalize it.
pub fn canon_path(path: &Path, fs: &impl FileOperations) -> Result<PathBuf> {
    fs.canonicalize(path).map_err(|_| {
        anyhow!(
            "failed to find {:?} with workdir {:?}",
            path,
            current_dir().unwrap()
        )
    })
}

/// Takes the set of all inputs and expands the globbed arguments.
///
/// # Examples
/// ```toml
/// [inputs.test_input]
/// arguments = [ "=glob=/test/**/*.jpg" ]
/// ```
///
/// May get expanded to:
///
/// ```toml
/// [inputs.test_input_glob_0]
/// arguments = [ "/test/a/a.jpg" ]
///
/// [inputs.test_input_glob_1]
/// arguments = [ "/test/a/b.jpg" ]
///
/// [inputs.test_input_glob_2]
/// arguments = [ "/test/b/b.jpg" ]
/// ```
pub fn expand_argument_globs(
    inputs: &BTreeMap<String, UserInput>,
) -> Result<BTreeMap<String, UserInput>> {
    let mut result = BTreeMap::new();

    for (original, input) in inputs {
        let mut globset = HashSet::new();
        globset.insert(input.clone());

        let mut is_glob = false;

        for arg_index in 0..input.arguments.len() {
            let mut next_globset = HashSet::new();

            for input_instance in &globset {
                is_glob |= explode_glob_set(input_instance, arg_index, &mut next_globset)?;
            }

            swap(&mut globset, &mut next_globset);
        }

        if is_glob {
            for (idx, glob) in globset.iter().enumerate() {
                result.insert(
                    format!("{}{}{}{}", original, INTERNAL_PREFIX, INTERNAL_GLOB, idx),
                    glob.clone(),
                );
            }
        } else {
            result.insert(original.clone(), input.clone());
        }
    }

    Ok(result)
}

/// Given a `input` and `arg_index` expand the glob at that
/// argument and put the results in `fill`.
fn explode_glob_set(
    input: &UserInput,
    arg_index: usize,
    fill: &mut HashSet<UserInput>,
) -> Result<bool> {
    let arg = &input.arguments[arg_index];
    let no_escape = arg.strip_prefix(GLOB_ESCAPE);

    if let Some(globbed) = no_escape {
        for path in glob(globbed).map_err(|_| {
            anyhow!(
                "could not expand the glob {globbed}, \
                any arguments starting with `{GLOB_ESCAPE}` are considered globs"
            )
        })? {
            let mut glob_instance = input.clone();

            glob_instance.arguments[arg_index] = path
                .map_err(|_| anyhow!("the glob failed to evaluate at {no_escape:?}"))?
                .to_str()
                .ok_or(anyhow!("failed to stringify {no_escape:?}"))?
                .to_string();

            fill.insert(glob_instance);
        }

        Ok(true)
    } else {
        fill.insert(input.clone());
        Ok(false)
    }
}
