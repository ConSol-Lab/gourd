use crate::config::UserInput;
use crate::experiment::{FieldRef, InternalInput};
use std::collections::BTreeMap;
use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RunInput {
    pub name: FieldRef,
    pub file: Option<PathBuf>,
    pub args: Vec<String>,
}

pub fn expand_input(_name: &String, _inp: &UserInput) -> Result<Vec<(String, InternalInput)>> {
    todo!()
}


pub fn iter_map<X: Ord, Y>(i: std::vec::IntoIter<(X, Y)>) -> BTreeMap<X, Y> {
    let mut map = BTreeMap::new();

    for (x, y) in i {
        map.insert(x, y);
    }

    map
}
