use core::fmt;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::env::current_dir;
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem::swap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use glob::glob;
use serde::de;
use serde::de::MapAccess;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;

use super::fetching::FetchedPath;
use super::Input;
use super::Program;
use crate::constants::GLOB_ESCAPE;
use crate::constants::INTERNAL_GLOB;
use crate::constants::INTERNAL_PREFIX;
use crate::file_system::FileOperations;
use crate::file_system::FileSystemInteractor;

/// A wrapper around [BTreeMap] to allow serde expansion of globs.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Serialize)]
pub struct InputMap(pub BTreeMap<String, Input>);

/// A wrapper around [BTreeMap] with programs.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Serialize)]
pub struct ProgramMap(pub BTreeMap<String, Program>);

impl<'de> Deserialize<'de> for ProgramMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// The custom map visitor for inputs.
        struct MapVisitor {
            /// Phantom marker.
            marker: PhantomData<()>,
        }

        impl<'de> Visitor<'de> for MapVisitor {
            type Value = ProgramMap;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut values = BTreeMap::new();

                while let Some((k, mut v)) = map.next_entry::<String, Program>()? {
                    if let DeserState::User(fs) = IS_USER_FACING.with_borrow(|x| x.clone()) {
                        v.binary = FetchedPath(canon_path(&v.binary, &fs)?);

                        if let Some(relative) = v.afterscript.clone() {
                            v.afterscript = Some(canon_path(&relative, &fs)?);
                        }

                        disallow_substring(&k, INTERNAL_PREFIX)?;
                    }

                    values.insert(k, v);
                }

                Ok(ProgramMap(values))
            }
        }

        let visitor = MapVisitor {
            marker: PhantomData,
        };

        deserializer.deserialize_map(visitor)
    }
}

impl<'de> Deserialize<'de> for InputMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// The custom map visitor for inputs.
        struct MapVisitor {
            /// Phantom marker.
            marker: PhantomData<()>,
        }

        impl<'de> Visitor<'de> for MapVisitor {
            type Value = InputMap;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map")
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut values = BTreeMap::new();

                while let Some((k, mut v)) = map.next_entry::<String, Input>()? {
                    if let DeserState::User(fs) = IS_USER_FACING.with_borrow(|x| x.clone()) {
                        if let Some(relative) = v.input.clone() {
                            v.input = Some(FetchedPath(canon_path(&relative, &fs)?));
                        }

                        disallow_substring(&k, INTERNAL_PREFIX)?;
                    }

                    values.insert(k, v);
                }

                let expanded = expand_globs(values)?;

                Ok(InputMap(expanded))
            }
        }

        let visitor = MapVisitor {
            marker: PhantomData,
        };

        deserializer.deserialize_map(visitor)
    }
}

/// This will take a path and canonicalize it.
fn canon_path<T>(path: &Path, fs: &impl FileOperations) -> Result<PathBuf, T>
where
    T: de::Error,
{
    fs.canonicalize(path).map_err(|_| {
        de::Error::custom(format!(
            "failed to find {:?} with workdir {:?}",
            path,
            current_dir().unwrap()
        ))
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
fn expand_globs<T>(inputs: BTreeMap<String, Input>) -> Result<BTreeMap<String, Input>, T>
where
    T: de::Error,
{
    let mut result = BTreeMap::new();

    for (original, input) in inputs {
        let mut globset = HashSet::new();
        globset.insert(input.clone());

        let mut is_glob = false;

        for arg_index in 0..input.arguments.len() {
            let mut next_globset = HashSet::new();

            for input_instance in &globset {
                is_glob |= explode_globset(input_instance, arg_index, &mut next_globset)?;
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
            result.insert(original, input);
        }
    }

    Ok(result)
}

/// Given a `input` and `arg_index` expand the glob at that
/// argument and put the results in `fill`.
fn explode_globset<T>(input: &Input, arg_index: usize, fill: &mut HashSet<Input>) -> Result<bool, T>
where
    T: de::Error,
{
    let arg = &input.arguments[arg_index];
    let no_escape = arg.strip_prefix(GLOB_ESCAPE);

    if let Some(globbed) = no_escape {
        for path in glob(globbed).map_err(|_| {
            de::Error::custom(format!(
                "could not expand the glob {globbed}, \
                any arguments starting with `{GLOB_ESCAPE}` are considered globs"
            ))
        })? {
            let mut glob_instance = input.clone();

            glob_instance.arguments[arg_index] = path
                .map_err(|_| {
                    de::Error::custom(format!("the glob failed to evaluate at {no_escape:?}"))
                })?
                .to_str()
                .ok_or(de::Error::custom(format!(
                    "failed to stringify {no_escape:?}"
                )))?
                .to_string();

            fill.insert(glob_instance);
        }

        Ok(true)
    } else {
        fill.insert(input.clone());
        Ok(false)
    }
}

/// Make sure that a substring is not part of a string.
fn disallow_substring<T>(name: &String, disallowed: &'static str) -> Result<(), T>
where
    T: de::Error,
{
    if name.contains(disallowed) {
        Err(de::Error::custom(format!(
            "failed to include the input {name}, \
            the input name contained `{disallowed}`, not allowed"
        )))
    } else {
        Ok(())
    }
}

impl Deref for InputMap {
    type Target = BTreeMap<String, Input>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for InputMap {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.0
    }
}

impl Deref for ProgramMap {
    type Target = BTreeMap<String, Program>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ProgramMap {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.0
    }
}

impl From<BTreeMap<String, Program>> for ProgramMap {
    fn from(value: BTreeMap<String, Program>) -> Self {
        ProgramMap(value)
    }
}

impl FromIterator<(String, Program)> for ProgramMap {
    fn from_iter<T: IntoIterator<Item = (String, Program)>>(iter: T) -> Self {
        ProgramMap(iter.into_iter().collect())
    }
}

impl IntoIterator for ProgramMap {
    type Item = (String, Program);
    type IntoIter = std::collections::btree_map::IntoIter<String, Program>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<BTreeMap<String, Input>> for InputMap {
    fn from(value: BTreeMap<String, Input>) -> Self {
        InputMap(value)
    }
}

impl FromIterator<(String, Input)> for InputMap {
    fn from_iter<T: IntoIterator<Item = (String, Input)>>(iter: T) -> Self {
        InputMap(iter.into_iter().collect())
    }
}

impl IntoIterator for InputMap {
    type Item = (String, Input);
    type IntoIter = std::collections::btree_map::IntoIter<String, Input>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
