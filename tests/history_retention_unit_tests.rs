#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::err_expect,
    clippy::too_many_arguments,
    clippy::needless_raw_string_hashes,
    unused_qualifications
)]

use sentinel::history::retention::{RetainLastNParams, retain_last_n_per_package_version};
use sentinel::history::types::{
    HistoryEvent, HistoryLockfileMetadata, HistoryPackageMetadata, HistoryRunMetadata,
};

fn make_event(
    event_id: &str,
    package_name: &str,
    package_version: &str,
    run_id: &str,
    occurred_at: &str,
) -> HistoryEvent {
    HistoryEvent {
        schema_version: 1,
        event_id: event_id.to_string(),
        run: HistoryRunMetadata {
            run_started_at: occurred_at.to_string(),
            run_id: run_id.to_string(),
        },
        occurred_at: occurred_at.to_string(),
        project_root: "/tmp/project".to_string(),
        package_manager: "npm".to_string(),
        command: "ci".to_string(),
        sentinel_version: "1.2.3".to_string(),
        lockfile: HistoryLockfileMetadata {
            path: "package-lock.json".to_string(),
            sha256_before: Some("before".to_string()),
            sha256_after: Some("after".to_string()),
        },
        package: HistoryPackageMetadata {
            name: package_name.to_string(),
            version: package_version.to_string(),
            direct: false,
        },
        result: "success".to_string(),
    }
}

#[test]
fn keeps_all_events_when_package_version_count_is_below_limit() {
    let events = vec![
        make_event("e1", "lodash", "4.17.21", "r1", "2026-04-22T10:00:00Z"),
        make_event("e2", "lodash", "4.17.21", "r2", "2026-04-22T10:05:00Z"),
    ];

    let retained = retain_last_n_per_package_version(RetainLastNParams {
        events: &events,
        max_per_key: 3,
    });

    assert_eq!(retained.len(), 2);
    assert_eq!(retained[0].event_id, "e1");
    assert_eq!(retained[1].event_id, "e2");
}

#[test]
fn keeps_only_last_three_events_for_single_package_version_fifo() {
    let events = vec![
        make_event("e1", "lodash", "4.17.21", "r1", "2026-04-22T10:00:00Z"),
        make_event("e2", "lodash", "4.17.21", "r2", "2026-04-22T10:05:00Z"),
        make_event("e3", "lodash", "4.17.21", "r3", "2026-04-22T10:10:00Z"),
        make_event("e4", "lodash", "4.17.21", "r4", "2026-04-22T10:15:00Z"),
        make_event("e5", "lodash", "4.17.21", "r5", "2026-04-22T10:20:00Z"),
    ];

    let retained = retain_last_n_per_package_version(RetainLastNParams {
        events: &events,
        max_per_key: 3,
    });

    assert_eq!(retained.len(), 3);
    assert_eq!(retained[0].event_id, "e3");
    assert_eq!(retained[1].event_id, "e4");
    assert_eq!(retained[2].event_id, "e5");
}

#[test]
fn applies_fifo_independently_per_package_and_version() {
    let events = vec![
        make_event("a1", "lodash", "4.17.21", "r1", "2026-04-22T10:00:00Z"),
        make_event("b1", "react", "18.3.1", "r1", "2026-04-22T10:01:00Z"),
        make_event("a2", "lodash", "4.17.21", "r2", "2026-04-22T10:02:00Z"),
        make_event("a3", "lodash", "4.17.21", "r3", "2026-04-22T10:03:00Z"),
        make_event("b2", "react", "18.3.1", "r2", "2026-04-22T10:04:00Z"),
        make_event("a4", "lodash", "4.17.21", "r4", "2026-04-22T10:05:00Z"),
    ];

    let retained = retain_last_n_per_package_version(RetainLastNParams {
        events: &events,
        max_per_key: 3,
    });
    let retained_ids: Vec<&str> = retained
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();

    assert_eq!(retained_ids, vec!["b1", "a2", "a3", "b2", "a4"]);
}

#[test]
fn package_versions_have_independent_fifo_windows() {
    let events = vec![
        make_event("v1", "lodash", "4.17.20", "r1", "2026-04-22T10:00:00Z"),
        make_event("v2", "lodash", "4.17.21", "r1", "2026-04-22T10:01:00Z"),
        make_event("v3", "lodash", "4.17.20", "r2", "2026-04-22T10:02:00Z"),
        make_event("v4", "lodash", "4.17.21", "r2", "2026-04-22T10:03:00Z"),
        make_event("v5", "lodash", "4.17.20", "r3", "2026-04-22T10:04:00Z"),
        make_event("v6", "lodash", "4.17.21", "r3", "2026-04-22T10:05:00Z"),
        make_event("v7", "lodash", "4.17.20", "r4", "2026-04-22T10:06:00Z"),
    ];

    let retained = retain_last_n_per_package_version(RetainLastNParams {
        events: &events,
        max_per_key: 3,
    });
    let retained_ids: Vec<&str> = retained
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();

    assert_eq!(retained_ids, vec!["v2", "v3", "v4", "v5", "v6", "v7"]);
}

#[test]
fn returns_empty_when_limit_is_zero() {
    let events = vec![make_event(
        "e1",
        "lodash",
        "4.17.21",
        "r1",
        "2026-04-22T10:00:00Z",
    )];

    let retained = retain_last_n_per_package_version(RetainLastNParams {
        events: &events,
        max_per_key: 0,
    });

    assert!(retained.is_empty());
}
