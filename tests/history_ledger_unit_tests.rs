#![allow(clippy::expect_used)]

use chrono::{Duration, Utc};
use sentinel::ecosystem::PackageManager;
use sentinel::history::ledger::{
    AppendHistoryEventsParams, HistoryQueryFilters, append_history_events, query_history_events,
};
use sentinel::history::path::{resolve_history_ledger_path, resolve_project_root};
use sentinel::history::types::HistoryPackageMetadata;

fn package(name: &str, version: &str, direct: bool) -> HistoryPackageMetadata {
    HistoryPackageMetadata {
        name: name.to_string(),
        version: version.to_string(),
        direct,
    }
}

#[test]
fn append_and_query_history_events_by_package() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");

    let packages = vec![package("lodash", "4.17.21", true)];
    let before = Some("before-sha".to_string());
    let after = Some("after-sha".to_string());

    let append_history_events_params = AppendHistoryEventsParams {
        current_working_directory: temp_dir.path(),
        package_manager: PackageManager::Npm,
        command: "install",
        lockfile_path: "package-lock.json",
        lock_hash_before: &before,
        lock_hash_after: &after,
        packages: &packages,
    };

    append_history_events(append_history_events_params).expect("history append should succeed");

    let project_root = resolve_project_root(temp_dir.path()).expect("project root should resolve");
    let ledger_path = resolve_history_ledger_path(&project_root);

    let filters = HistoryQueryFilters {
        from: Utc::now() - Duration::minutes(5),
        to: Utc::now() + Duration::minutes(5),
        package: Some("lodash".to_string()),
        version: Some("4.17.21".to_string()),
        project: None,
        package_manager: Some("npm".to_string()),
    };

    let events = query_history_events(&ledger_path, &filters).expect("query should succeed");

    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.command, "install");
    assert_eq!(event.package.name, "lodash");
    assert_eq!(event.package.version, "4.17.21");
    assert_eq!(event.lockfile.path, "package-lock.json");
}

#[test]
fn query_fails_when_first_line_is_corrupted() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let project_root = resolve_project_root(temp_dir.path()).expect("project root should resolve");
    let ledger_path = resolve_history_ledger_path(&project_root);

    std::fs::create_dir_all(
        ledger_path
            .parent()
            .expect("history ledger should have parent directory"),
    )
    .expect("history directory should be created");

    std::fs::write(&ledger_path, "{invalid-json\n")
        .expect("corrupted history ledger should be written");

    let filters = HistoryQueryFilters {
        from: Utc::now() - Duration::minutes(1),
        to: Utc::now() + Duration::minutes(1),
        package: None,
        version: None,
        project: None,
        package_manager: None,
    };

    let query_result = query_history_events(&ledger_path, &filters);

    assert!(query_result.is_err());
    let error_message = query_result.expect_err("query should fail with an error message");
    assert!(error_message.contains("corrupted"));
}

#[test]
fn query_missing_ledger_returns_actionable_hint() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let missing_ledger_path = temp_dir
        .path()
        .join(".sentinel")
        .join("install-history.ndjson");

    let filters = HistoryQueryFilters {
        from: Utc::now() - Duration::minutes(1),
        to: Utc::now() + Duration::minutes(1),
        package: None,
        version: None,
        project: None,
        package_manager: None,
    };

    let query_result = query_history_events(&missing_ledger_path, &filters);

    assert!(query_result.is_err());
    let error_message = query_result.expect_err("query should fail with missing ledger hint");
    assert!(error_message.contains("install history ledger not found at"));
    assert!(error_message.contains("without --dry-run"));
}
