use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::config::Label;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Labels {
    /// The labels of the experiment.
    pub labels: BTreeMap<String, Label>,

    /// Throw an error when multiple labels are present in afterscript output.
    pub warn_on_label_overlap: bool,
}