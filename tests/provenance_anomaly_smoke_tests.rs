#![allow(clippy::expect_used)]

use sentinel::ecosystem::PackageManager;
use sentinel::history::ledger::{AppendHistoryEventsParams, append_history_events};
use sentinel::history::path::{resolve_history_ledger_path, resolve_project_root};
use sentinel::history::types::{HistoryEventPackageInput, HistoryPackageMetadata};
use sentinel::types::{ProvenanceAnomaly, ProvenanceAnomalyCheckParams};
use sentinel::verifier::provenance_anomaly::check_provenance_anomalies;

use chrono::{Duration, Utc};
use sentinel::history::ledger::{HistoryQueryFilters, query_history_events};

#[allow(clippy::too_many_arguments)]
fn append_package_event(
    dir: &std::path::Path,
    name: &str,
    version: &str,
    had_provenance: bool,
    workflow_path: Option<&str>,
) {
    let packages = vec![HistoryEventPackageInput {
        package: HistoryPackageMetadata {
            name: name.to_string(),
            version: version.to_string(),
            direct: true,
        },
        had_provenance,
        provenance_workflow_path: workflow_path.map(ToString::to_string),
    }];

    let append_params = AppendHistoryEventsParams {
        current_working_directory: dir,
        package_manager: PackageManager::Npm,
        command: "ci",
        lockfile_path: "package-lock.json",
        lock_hash_before: &None,
        lock_hash_after: &None,
        packages: &packages,
    };

    append_history_events(append_params).expect("append should succeed");
}

#[test]
fn smoke_provenance_disappeared_via_ledger() {
    let temp_dir = tempfile::tempdir().expect("tempdir");

    append_package_event(
        temp_dir.path(),
        "axios",
        "1.6.0",
        true,
        Some(".github/workflows/publish.yml"),
    );

    let project_root = resolve_project_root(temp_dir.path()).expect("project root");
    let ledger_path = resolve_history_ledger_path(&project_root);
    let filters = HistoryQueryFilters {
        from: Utc::now() - Duration::minutes(5),
        to: Utc::now() + Duration::minutes(5),
        package: Some("axios".to_string()),
        version: None,
        project: None,
        package_manager: None,
    };
    let events = query_history_events(&ledger_path, &filters).expect("query");
    let last_event = events.last().expect("should have event");

    let check_params = ProvenanceAnomalyCheckParams {
        current_had_provenance: false,
        current_workflow_path: None,
        last_event_for_package: Some(last_event),
    };

    let anomalies = check_provenance_anomalies(&check_params);

    assert_eq!(anomalies.len(), 1);
    assert_eq!(anomalies[0], ProvenanceAnomaly::ProvenanceDisappeared);
}

#[test]
fn smoke_workflow_changed_via_ledger() {
    let temp_dir = tempfile::tempdir().expect("tempdir");

    append_package_event(
        temp_dir.path(),
        "tanstack-query",
        "5.0.0",
        true,
        Some(".github/workflows/release.yml"),
    );

    let project_root = resolve_project_root(temp_dir.path()).expect("project root");
    let ledger_path = resolve_history_ledger_path(&project_root);
    let filters = HistoryQueryFilters {
        from: Utc::now() - Duration::minutes(5),
        to: Utc::now() + Duration::minutes(5),
        package: Some("tanstack-query".to_string()),
        version: None,
        project: None,
        package_manager: None,
    };
    let events = query_history_events(&ledger_path, &filters).expect("query");
    let last_event = events.last().expect("should have event");

    let check_params = ProvenanceAnomalyCheckParams {
        current_had_provenance: true,
        current_workflow_path: Some(".github/workflows/injected-build.yml"),
        last_event_for_package: Some(last_event),
    };

    let anomalies = check_provenance_anomalies(&check_params);

    assert_eq!(anomalies.len(), 1);
    assert_eq!(
        anomalies[0],
        ProvenanceAnomaly::WorkflowChanged {
            previous: ".github/workflows/release.yml".to_string(),
            current: ".github/workflows/injected-build.yml".to_string(),
        }
    );
}

#[test]
fn smoke_cold_start_no_false_positive() {
    let check_params = ProvenanceAnomalyCheckParams {
        current_had_provenance: true,
        current_workflow_path: Some(".github/workflows/release.yml"),
        last_event_for_package: None,
    };

    let anomalies = check_provenance_anomalies(&check_params);

    assert!(
        anomalies.is_empty(),
        "cold-start must not produce false positives"
    );
}

#[test]
fn smoke_old_ledger_entries_do_not_trigger_false_anomalies() {
    let temp_dir = tempfile::tempdir().expect("tempdir");

    append_package_event(temp_dir.path(), "react", "18.2.0", false, None);

    let project_root = resolve_project_root(temp_dir.path()).expect("project root");
    let ledger_path = resolve_history_ledger_path(&project_root);
    let filters = HistoryQueryFilters {
        from: Utc::now() - Duration::minutes(5),
        to: Utc::now() + Duration::minutes(5),
        package: Some("react".to_string()),
        version: None,
        project: None,
        package_manager: None,
    };
    let events = query_history_events(&ledger_path, &filters).expect("query");
    let last_event = events.last().expect("should have event");

    let check_params = ProvenanceAnomalyCheckParams {
        current_had_provenance: false,
        current_workflow_path: None,
        last_event_for_package: Some(last_event),
    };

    let anomalies = check_provenance_anomalies(&check_params);

    assert!(anomalies.is_empty());
}
