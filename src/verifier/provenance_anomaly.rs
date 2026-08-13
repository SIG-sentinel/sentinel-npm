use crate::types::{ProvenanceAnomaly, ProvenanceAnomalyCheckParams};

pub fn check_provenance_anomalies(
    params: &ProvenanceAnomalyCheckParams<'_>,
) -> Vec<ProvenanceAnomaly> {
    let ProvenanceAnomalyCheckParams {
        current_had_provenance,
        current_workflow_path,
        last_event_for_package,
    } = params;

    let Some(last_event) = last_event_for_package else {
        return Vec::new();
    };

    let mut anomalies = Vec::new();
    let previous_had_provenance = last_event.had_provenance;
    let provenance_disappeared = previous_had_provenance && !current_had_provenance;

    if provenance_disappeared {
        anomalies.push(ProvenanceAnomaly::ProvenanceDisappeared);
    }

    let previous_workflow_path = last_event.provenance_workflow_path.as_deref();

    let workflow_changed_anomaly = match (previous_workflow_path, *current_workflow_path) {
        (Some(previous), Some(current)) if previous != current => {
            Some(ProvenanceAnomaly::WorkflowChanged {
                previous: previous.to_string(),
                current: current.to_string(),
            })
        }
        _ => None,
    };

    if let Some(workflow_changed_anomaly) = workflow_changed_anomaly {
        anomalies.push(workflow_changed_anomaly);
    }

    anomalies
}
