use std::collections::{HashMap, HashSet, VecDeque};

use crate::history::types::HistoryEvent;

pub use crate::types::RetainLastNParams;

const RETENTION_DISABLED_LIMIT: usize = 0;

pub fn retain_last_n_per_package_version(params: RetainLastNParams<'_>) -> Vec<HistoryEvent> {
    let RetainLastNParams {
        events,
        max_per_key,
    } = params;
    let has_no_retention_window = max_per_key == RETENTION_DISABLED_LIMIT;

    if has_no_retention_window {
        return Vec::new();
    }

    let mut kept_indices_by_key: HashMap<(String, String), VecDeque<usize>> = HashMap::new();

    for (index, event) in events.iter().enumerate() {
        let package_name = event.package.name.clone();
        let package_version = event.package.version.clone();
        let key = (package_name, package_version);

        let entry = kept_indices_by_key.entry(key).or_default();

        entry.push_back(index);

        let exceeds_retention_limit = entry.len() > max_per_key;

        if exceeds_retention_limit {
            entry.pop_front();
        }
    }

    let kept_indices: HashSet<usize> = kept_indices_by_key
        .into_values()
        .flat_map(IntoIterator::into_iter)
        .collect();

    events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| kept_indices.contains(&index).then_some(event.clone()))
        .collect()
}
