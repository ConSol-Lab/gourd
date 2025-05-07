use anyhow::Result;
use gourd_lib::experiment::Experiment;
use log::debug;
use log::trace;
use log::warn;

/// Assigns a label to a run.
pub fn assign_label(
    run_id: usize,
    source_text: &str,
    experiment: &Experiment,
) -> Result<Option<String>> {
    debug!("Assigning label for text {source_text:?}");

    let mut result_label: Option<String> = None;

    let label_map = &experiment.labels;
    let mut keys = label_map.keys().collect::<Vec<&String>>();
    keys.sort_unstable_by(|a, b| label_map[*b].priority.cmp(&label_map[*a].priority));

    for l in keys {
        let label = &label_map[l];
        if label.regex.is_match(source_text) {
            if let Some(ref r) = result_label {
                warn!("The afterscript for run {run_id:?} matches multiple labels: {r} and {l}");
            } else {
                trace!("{source_text} matches {l}");
                result_label = Some(l.clone());
            }
        }
    }

    Ok(result_label)
}
