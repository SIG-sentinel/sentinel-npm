use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::constants::{
    LEDGER_ERR_APPEND_EVENT, LEDGER_ERR_APPEND_NEWLINE, LEDGER_ERR_CORRUPTED_FIRST_LINE_TEMPLATE,
    LEDGER_ERR_CREATE_COMPACTED, LEDGER_ERR_CREATE_DIRECTORY, LEDGER_ERR_CREATE_FILE,
    LEDGER_ERR_INSPECT_BEFORE_COMPACTION, LEDGER_ERR_INSPECT_METADATA,
    LEDGER_ERR_NOT_FOUND_TEMPLATE, LEDGER_ERR_OPEN_FILE, LEDGER_ERR_OPEN_FOR_APPEND,
    LEDGER_ERR_OPEN_FOR_COMPACTION, LEDGER_ERR_READ_DURING_COMPACTION, LEDGER_ERR_READ_FILE,
    LEDGER_ERR_READ_FIRST_LINE, LEDGER_ERR_READ_LINE, LEDGER_ERR_REPLACE_WITH_COMPACTED,
    LEDGER_ERR_RESOLVE_PROJECT_ROOT, LEDGER_ERR_SERIALIZE_DURING_COMPACTION,
    LEDGER_ERR_SERIALIZE_EVENT, LEDGER_ERR_WRITE_COMPACTED, LEDGER_ERR_WRITE_COMPACTED_NEWLINE,
    LEDGER_MISSING_TIP, LEDGER_WARN_SKIPPED_LINES_TEMPLATE, PACKAGE_AT_VERSION_TEMPLATE,
    RUN_ID_TEMPLATE, SENTINEL_HISTORY_MAX_BYTES, TEMP_COMPACTION_EXTENSION_TEMPLATE,
    render_template,
};
use crate::history::path::{resolve_history_ledger_path, resolve_project_root};
use crate::history::retention::{RetainLastNParams, retain_last_n_per_package_version};
use crate::history::types::{HistoryEvent, HistoryLockfileMetadata, HistoryRunMetadata};

pub use crate::types::{
    AppendEventsImplParams, AppendHistoryEventsParams, EventMatchesFiltersParams,
    HistoryQueryFilters,
};

const HISTORY_SCHEMA_VERSION: u32 = 1;
const HISTORY_KEEP_LAST_PER_PACKAGE_VERSION: usize = 3;
const HISTORY_RESULT_SUCCESS: &str = "success";
const EMPTY_LEDGER_BYTES: u64 = 0;
const FIRST_LEDGER_LINE_INDEX: usize = 0;

fn parse_rfc3339_utc(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn matches_optional_string_filter(filter: Option<&String>, value: &str) -> bool {
    filter.is_none_or(|expected| value == expected)
}

fn matches_optional_ascii_case_filter(filter: Option<&String>, value: &str) -> bool {
    filter.is_none_or(|expected| value.eq_ignore_ascii_case(expected))
}

fn event_matches_filters(params: &EventMatchesFiltersParams<'_>) -> bool {
    let EventMatchesFiltersParams {
        event,
        occurred_at,
        filters,
    } = *params;
    let is_before_from = occurred_at < filters.from;
    let is_after_to = occurred_at > filters.to;
    let is_outside_range = is_before_from || is_after_to;

    let package_matches =
        matches_optional_string_filter(filters.package.as_ref(), &event.package.name);

    let version_matches =
        matches_optional_string_filter(filters.version.as_ref(), &event.package.version);

    let project_matches =
        matches_optional_string_filter(filters.project.as_ref(), &event.project_root);

    let package_manager_matches = matches_optional_ascii_case_filter(
        filters.package_manager.as_ref(),
        &event.package_manager,
    );

    !is_outside_range
        && package_matches
        && version_matches
        && project_matches
        && package_manager_matches
}

pub fn append_history_events(params: AppendHistoryEventsParams<'_>) -> Result<(), String> {
    let AppendHistoryEventsParams {
        current_working_directory,
        package_manager,
        command,
        lockfile_path,
        lock_hash_before,
        lock_hash_after,
        packages,
    } = params;

    if packages.is_empty() {
        return Ok(());
    }

    let project_root = resolve_project_root(current_working_directory)
        .map_err(|error| render_template(LEDGER_ERR_RESOLVE_PROJECT_ROOT, &[error.to_string()]))?;

    let ledger_path = resolve_history_ledger_path(&project_root);

    ensure_ledger_ready(&ledger_path)?;
    maybe_compact_ledger(&ledger_path)?;

    let run_started_at = Utc::now().to_rfc3339();
    let run_id = build_run_id();

    let events: Vec<HistoryEvent> = packages
        .iter()
        .enumerate()
        .map(|(index, package)| HistoryEvent {
            schema_version: HISTORY_SCHEMA_VERSION,
            event_id: render_template(RUN_ID_TEMPLATE, &[run_id.clone(), index.to_string()]),
            run: HistoryRunMetadata {
                run_started_at: run_started_at.clone(),
                run_id: run_id.clone(),
            },
            occurred_at: Utc::now().to_rfc3339(),
            project_root: project_root.to_string_lossy().to_string(),
            package_manager: package_manager.command().to_string(),
            command: command.to_string(),
            sentinel_version: env!("CARGO_PKG_VERSION").to_string(),
            lockfile: HistoryLockfileMetadata {
                path: lockfile_path.to_string(),
                sha256_before: lock_hash_before.clone(),
                sha256_after: lock_hash_after.clone(),
            },
            package: package.clone(),
            result: HISTORY_RESULT_SUCCESS.to_string(),
        })
        .collect();

    let append_events_params = AppendEventsImplParams {
        ledger_path: &ledger_path,
        events: &events,
    };
    append_events(append_events_params)
}

pub fn query_history_events(
    ledger_path: &Path,
    filters: &HistoryQueryFilters,
) -> Result<Vec<HistoryEvent>, String> {
    if !ledger_path.exists() {
        return Err(render_template(
            LEDGER_ERR_NOT_FOUND_TEMPLATE,
            &[
                ledger_path.display().to_string(),
                LEDGER_MISSING_TIP.to_string(),
            ],
        ));
    }

    let file = std::fs::File::open(ledger_path)
        .map_err(|error| render_template(LEDGER_ERR_READ_FILE, &[error.to_string()]))?;
    let reader = BufReader::new(file);

    let mut events = Vec::new();
    let mut skipped_lines = 0usize;

    for (line_index, line_result) in reader.lines().enumerate() {
        let line = line_result
            .map_err(|error| render_template(LEDGER_ERR_READ_LINE, &[error.to_string()]))?;

        if line.trim().is_empty() {
            continue;
        }

        let event = match serde_json::from_str::<HistoryEvent>(&line) {
            Ok(event) => event,
            Err(error) if line_index == FIRST_LEDGER_LINE_INDEX => {
                return Err(render_template(
                    LEDGER_ERR_CORRUPTED_FIRST_LINE_TEMPLATE,
                    &[error.to_string()],
                ));
            }
            Err(_) => {
                skipped_lines += 1;

                continue;
            }
        };

        let Some(occurred_at) = parse_rfc3339_utc(&event.occurred_at) else {
            skipped_lines += 1;

            continue;
        };

        let event_matches_filters_params = EventMatchesFiltersParams {
            event: &event,
            occurred_at,
            filters,
        };
        let matches_filters = event_matches_filters(&event_matches_filters_params);

        if !matches_filters {
            continue;
        }

        events.push(event);
    }

    events.sort_by(|left, right| left.occurred_at.cmp(&right.occurred_at));

    let has_skipped_lines = skipped_lines > 0;

    if has_skipped_lines {
        eprintln!(
            "{}",
            render_template(
                LEDGER_WARN_SKIPPED_LINES_TEMPLATE,
                &[skipped_lines.to_string(), ledger_path.display().to_string()],
            )
        );
    }

    Ok(events)
}

pub fn summarize_unique_packages(events: &[HistoryEvent]) -> Vec<String> {
    let unique: HashSet<String> = events
        .iter()
        .map(|event| {
            render_template(
                PACKAGE_AT_VERSION_TEMPLATE,
                &[event.package.name.clone(), event.package.version.clone()],
            )
        })
        .collect();

    let mut packages: Vec<String> = unique.into_iter().collect();

    packages.sort();
    packages
}

pub fn count_unique_projects(events: &[HistoryEvent]) -> usize {
    let projects: HashSet<&str> = events
        .iter()
        .map(|event| event.project_root.as_str())
        .collect();

    projects.len()
}

fn ensure_ledger_ready(ledger_path: &Path) -> Result<(), String> {
    ledger_path
        .parent()
        .map(|parent| {
            std::fs::create_dir_all(parent)
                .map_err(|error| render_template(LEDGER_ERR_CREATE_DIRECTORY, &[error.to_string()]))
        })
        .transpose()?;

    if !ledger_path.exists() {
        std::fs::File::create(ledger_path)
            .map_err(|error| render_template(LEDGER_ERR_CREATE_FILE, &[error.to_string()]))?;

        return Ok(());
    }

    let metadata = std::fs::metadata(ledger_path)
        .map_err(|error| render_template(LEDGER_ERR_INSPECT_METADATA, &[error.to_string()]))?;

    if metadata.len() == EMPTY_LEDGER_BYTES {
        return Ok(());
    }

    validate_first_line(ledger_path)
}

fn validate_first_line(ledger_path: &Path) -> Result<(), String> {
    let file = std::fs::File::open(ledger_path)
        .map_err(|error| render_template(LEDGER_ERR_OPEN_FILE, &[error.to_string()]))?;

    let mut reader = BufReader::new(file);
    let mut first_line = String::new();

    reader
        .read_line(&mut first_line)
        .map_err(|error| render_template(LEDGER_ERR_READ_FIRST_LINE, &[error.to_string()]))?;

    if first_line.trim().is_empty() {
        return Ok(());
    }

    serde_json::from_str::<HistoryEvent>(&first_line)
        .map(|_| ())
        .map_err(|error| {
            render_template(
                LEDGER_ERR_CORRUPTED_FIRST_LINE_TEMPLATE,
                &[error.to_string()],
            )
        })
}

fn maybe_compact_ledger(ledger_path: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(ledger_path).map_err(|error| {
        render_template(LEDGER_ERR_INSPECT_BEFORE_COMPACTION, &[error.to_string()])
    })?;

    let is_within_size_limit = metadata.len() < SENTINEL_HISTORY_MAX_BYTES;

    if is_within_size_limit {
        return Ok(());
    }

    compact_ledger(ledger_path)
}

fn compact_ledger(ledger_path: &Path) -> Result<(), String> {
    let file = std::fs::File::open(ledger_path)
        .map_err(|error| render_template(LEDGER_ERR_OPEN_FOR_COMPACTION, &[error.to_string()]))?;
    let reader = BufReader::new(file);

    let mut events = Vec::new();

    reader.lines().try_for_each(|line_result| {
        let line = line_result.map_err(|error| {
            render_template(LEDGER_ERR_READ_DURING_COMPACTION, &[error.to_string()])
        })?;

        let is_empty_line = line.trim().is_empty();

        if is_empty_line {
            return Ok(());
        }

        if let Ok(event) = serde_json::from_str::<HistoryEvent>(&line) {
            events.push(event);
        }

        Ok::<(), String>(())
    })?;

    let retain_last_n_params = RetainLastNParams {
        events: &events,
        max_per_key: HISTORY_KEEP_LAST_PER_PACKAGE_VERSION,
    };
    let retained = retain_last_n_per_package_version(retain_last_n_params);
    let temp_path = temp_compaction_path(ledger_path);

    {
        let mut file = std::fs::File::create(&temp_path)
            .map_err(|error| render_template(LEDGER_ERR_CREATE_COMPACTED, &[error.to_string()]))?;

        retained.into_iter().try_for_each(|event| {
            let line = serde_json::to_string(&event).map_err(|error| {
                render_template(LEDGER_ERR_SERIALIZE_DURING_COMPACTION, &[error.to_string()])
            })?;

            file.write_all(line.as_bytes()).map_err(|error| {
                render_template(LEDGER_ERR_WRITE_COMPACTED, &[error.to_string()])
            })?;

            file.write_all(b"\n").map_err(|error| {
                render_template(LEDGER_ERR_WRITE_COMPACTED_NEWLINE, &[error.to_string()])
            })
        })?;
    }

    std::fs::rename(&temp_path, ledger_path)
        .map_err(|error| render_template(LEDGER_ERR_REPLACE_WITH_COMPACTED, &[error.to_string()]))
}

fn append_events(params: AppendEventsImplParams<'_>) -> Result<(), String> {
    let AppendEventsImplParams {
        ledger_path,
        events,
    } = params;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger_path)
        .map_err(|error| render_template(LEDGER_ERR_OPEN_FOR_APPEND, &[error.to_string()]))?;

    events.iter().try_for_each(|event| {
        let line = serde_json::to_string(event)
            .map_err(|error| render_template(LEDGER_ERR_SERIALIZE_EVENT, &[error.to_string()]))?;

        file.write_all(line.as_bytes())
            .map_err(|error| render_template(LEDGER_ERR_APPEND_EVENT, &[error.to_string()]))?;

        file.write_all(b"\n")
            .map_err(|error| render_template(LEDGER_ERR_APPEND_NEWLINE, &[error.to_string()]))
    })?;

    Ok(())
}

fn temp_compaction_path(ledger_path: &Path) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    ledger_path.with_extension(render_template(
        TEMP_COMPACTION_EXTENSION_TEMPLATE,
        &[nanos.to_string()],
    ))
}

fn build_run_id() -> String {
    let process_id = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    render_template(
        RUN_ID_TEMPLATE,
        &[process_id.to_string(), nanos.to_string()],
    )
}
