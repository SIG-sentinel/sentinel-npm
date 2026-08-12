#![allow(clippy::expect_used)]

use sentinel::history::types::{
    HistoryEvent, HistoryLockfileMetadata, HistoryPackageMetadata, HistoryRunMetadata,
};
use sentinel::types::{ProvenanceAnomaly, ProvenanceAnomalyCheckParams};
use sentinel::verifier::provenance_anomaly::check_provenance_anomalies;

fn make_history_event(
    package_name: &str,
    version: &str,
    had_provenance: bool,
    workflow_path: Option<&str>,
) -> HistoryEvent {
    HistoryEvent {
        schema_version: 1,
        event_id: "test-event-001".to_string(),
        run: HistoryRunMetadata {
            run_started_at: "2026-06-01T10:00:00Z".to_string(),
            run_id: "run-001".to_string(),
        },
        occurred_at: "2026-06-01T10:00:00Z".to_string(),
        project_root: "/tmp/test-project".to_string(),
        package_manager: "npm".to_string(),
        command: "ci".to_string(),
        sentinel_version: "2.2.0".to_string(),
        lockfile: HistoryLockfileMetadata {
            path: "package-lock.json".to_string(),
            sha256_before: None,
            sha256_after: None,
        },
        package: HistoryPackageMetadata {
            name: package_name.to_string(),
            version: version.to_string(),
            direct: true,
        },
        result: "success".to_string(),
        had_provenance,
        provenance_workflow_path: workflow_path.map(ToString::to_string),
    }
}

#[test]
fn no_anomaly_when_no_history() {
    let params = ProvenanceAnomalyCheckParams {
        current_had_provenance: true,
        current_workflow_path: Some(".github/workflows/release.yml"),
        last_event_for_package: None,
    };

    let anomalies = check_provenance_anomalies(&params);

    assert!(anomalies.is_empty());
}

#[test]
fn no_anomaly_when_provenance_stays_present() {
    let last_event = make_history_event(
        "tanstack-query",
        "5.0.0",
        true,
        Some(".github/workflows/release.yml"),
    );

    let params = ProvenanceAnomalyCheckParams {
        current_had_provenance: true,
        current_workflow_path: Some(".github/workflows/release.yml"),
        last_event_for_package: Some(&last_event),
    };

    let anomalies = check_provenance_anomalies(&params);

    assert!(anomalies.is_empty());
}

#[test]
fn no_anomaly_when_provenance_was_never_present() {
    let last_event = make_history_event("lodash", "4.17.21", false, None);

    let params = ProvenanceAnomalyCheckParams {
        current_had_provenance: false,
        current_workflow_path: None,
        last_event_for_package: Some(&last_event),
    };

    let anomalies = check_provenance_anomalies(&params);

    assert!(anomalies.is_empty());
}

#[test]
fn detects_provenance_disappeared() {
    let last_event = make_history_event(
        "axios",
        "1.6.0",
        true,
        Some(".github/workflows/publish.yml"),
    );

    let params = ProvenanceAnomalyCheckParams {
        current_had_provenance: false,
        current_workflow_path: None,
        last_event_for_package: Some(&last_event),
    };

    let anomalies = check_provenance_anomalies(&params);

    assert_eq!(anomalies.len(), 1);
    assert_eq!(anomalies[0], ProvenanceAnomaly::ProvenanceDisappeared);
}

#[test]
fn no_anomaly_when_workflow_unchanged() {
    let last_event = make_history_event(
        "tanstack-query",
        "5.0.0",
        true,
        Some(".github/workflows/release.yml"),
    );

    let params = ProvenanceAnomalyCheckParams {
        current_had_provenance: true,
        current_workflow_path: Some(".github/workflows/release.yml"),
        last_event_for_package: Some(&last_event),
    };

    let anomalies = check_provenance_anomalies(&params);

    assert!(anomalies.is_empty());
}

#[test]
fn detects_workflow_path_changed() {
    let last_event = make_history_event(
        "tanstack-query",
        "5.0.0",
        true,
        Some(".github/workflows/release.yml"),
    );

    let params = ProvenanceAnomalyCheckParams {
        current_had_provenance: true,
        current_workflow_path: Some(".github/workflows/compromised.yml"),
        last_event_for_package: Some(&last_event),
    };

    let anomalies = check_provenance_anomalies(&params);

    assert_eq!(anomalies.len(), 1);
    assert_eq!(
        anomalies[0],
        ProvenanceAnomaly::WorkflowChanged {
            previous: ".github/workflows/release.yml".to_string(),
            current: ".github/workflows/compromised.yml".to_string(),
        }
    );
}

#[test]
fn no_workflow_anomaly_when_previous_had_no_workflow() {
    let last_event = make_history_event("some-pkg", "1.0.0", true, None);

    let params = ProvenanceAnomalyCheckParams {
        current_had_provenance: true,
        current_workflow_path: Some(".github/workflows/release.yml"),
        last_event_for_package: Some(&last_event),
    };

    let anomalies = check_provenance_anomalies(&params);

    assert!(anomalies.is_empty());
}

#[test]
fn no_workflow_anomaly_when_current_has_no_workflow() {
    let last_event = make_history_event(
        "some-pkg",
        "1.0.0",
        true,
        Some(".github/workflows/release.yml"),
    );

    let params = ProvenanceAnomalyCheckParams {
        current_had_provenance: true,
        current_workflow_path: None,
        last_event_for_package: Some(&last_event),
    };

    let anomalies = check_provenance_anomalies(&params);

    assert!(anomalies.is_empty());
}

#[test]
fn detects_provenance_disappeared_when_previous_workflow_was_known() {
    let last_event = make_history_event(
        "axios",
        "1.6.0",
        true,
        Some(".github/workflows/publish.yml"),
    );

    let params = ProvenanceAnomalyCheckParams {
        current_had_provenance: false,
        current_workflow_path: None,
        last_event_for_package: Some(&last_event),
    };

    let anomalies = check_provenance_anomalies(&params);

    assert_eq!(anomalies.len(), 1);
    assert_eq!(anomalies[0], ProvenanceAnomaly::ProvenanceDisappeared);
}

#[test]
fn history_event_with_new_fields_serializes_and_deserializes() {
    let event = make_history_event(
        "tanstack-query",
        "5.1.0",
        true,
        Some(".github/workflows/release.yml"),
    );

    let json = serde_json::to_string(&event).expect("should serialize");
    let deserialized: HistoryEvent = serde_json::from_str(&json).expect("should deserialize");

    assert!(deserialized.had_provenance);
    assert_eq!(
        deserialized.provenance_workflow_path.as_deref(),
        Some(".github/workflows/release.yml")
    );
}

#[test]
fn old_ledger_entry_without_new_fields_deserializes_with_defaults() {
    let old_json = r#"{
        "schema_version": 1,
        "event_id": "old-001",
        "run": {"run_started_at": "2025-01-01T00:00:00Z", "run_id": "run-old"},
        "occurred_at": "2025-01-01T00:00:00Z",
        "project_root": "/tmp/old",
        "package_manager": "npm",
        "command": "ci",
        "sentinel_version": "2.0.0",
        "lockfile": {"path": "package-lock.json", "sha256_before": null, "sha256_after": null},
        "package": {"name": "lodash", "version": "4.17.21", "direct": true},
        "result": "success"
    }"#;

    let event: HistoryEvent =
        serde_json::from_str(old_json).expect("old format should deserialize");

    assert!(!event.had_provenance);
    assert_eq!(event.provenance_workflow_path, None);
}
