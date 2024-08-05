use std::collections::BTreeMap;
use std::collections::BTreeSet;

use anyhow::Result;

use crate::config::UserProgram;
use crate::config::UserProgramMap;
use crate::experiment::InternalProgram;



pub fn expand_program(_name: &String, _prog: &UserProgram) -> Result<Vec<(String, InternalProgram)>> {
    todo!()
}


pub trait Child<X> {
    fn parents(&self) -> Vec<X>;
}

impl Child<String> for UserProgram {
    fn parents(&self) -> Vec<String> {
        self.runs_after.clone().unwrap_or_default()
    }
}

pub fn topological_ordering<X: Ord, Y: Child<X>>(_map: BTreeMap<X, Y>) -> Result<Vec<(X, Y)>> {
    // let mut stack = vec![];
    // let mut visited: BTreeSet<X> = BTreeSet::new();
    todo!();

    // Ok(stack)
}
//
// fn visit<X: Ord, Y: Child<X>>(node: X, map: BTreeMap<X, Y>, stack: &mut
// Vec<(X, Y)>, visited: &mut BTreeSet<X>) {
//
// }
