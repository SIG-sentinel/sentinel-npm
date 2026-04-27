use std::process::ExitCode;

use chrono::Utc;

use crate::constants::{
    HISTORY_ERR_INVALID_FROM_TIMESTAMP, HISTORY_ERR_INVALID_RANGE_FROM_GT_TO,
    HISTORY_ERR_INVALID_TO_TIMESTAMP, HISTORY_ERR_RESOLVE_CWD, HISTORY_ERR_RESOLVE_PROJECT,
};
use crate::history::ledger::{
    count_unique_projects, query_history_events, summarize_unique_packages,
};
use crate::history::path::{resolve_history_ledger_path, resolve_project_root};
use crate::types::{
    HistoryArgs, HistoryPackageModeOutput, HistoryQuery, HistoryQueryFilters,
    HistoryRangeModeOutput, HistoryRangeSummary, HistoryRunMode, RenderPackageModeParams,
    RenderRangeModeParams,
};
use crate::ui::command_feedback as ui;
use crate::ui::history_output;
use crate::utils::format_err_with_reason;

const EXIT_CODE_INVALID_INPUT: u8 = 2;

fn parse_rfc3339_utc(value: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn classify_history_run_mode(package_filter: Option<&String>) -> HistoryRunMode {
    match package_filter {
        Some(_) => HistoryRunMode::Package,
        None => HistoryRunMode::Range,
    }
}

fn resolve_project_filter(
    project_filter: Option<&std::path::PathBuf>,
) -> Result<Option<String>, ExitCode> {
    let project_filter_result = project_filter
        .map(|project_path| {
            resolve_project_root(project_path)
                .map(|path| path.to_string_lossy().to_string())
                .map_err(|error| format_err_with_reason(HISTORY_ERR_RESOLVE_PROJECT, &error))
        })
        .transpose();

    match project_filter_result {
        Ok(project_filter) => Ok(project_filter),
        Err(error_message) => {
            ui::print_generic_error(&error_message);
            Err(ExitCode::from(EXIT_CODE_INVALID_INPUT))
        }
    }
}

#[allow(clippy::unused_async)]
pub async fn run(args: &HistoryArgs) -> ExitCode {
    let Some(from_timestamp) = parse_rfc3339_utc(&args.from) else {
        ui::print_generic_error(HISTORY_ERR_INVALID_FROM_TIMESTAMP);

        return ExitCode::from(EXIT_CODE_INVALID_INPUT);
    };

    let Some(to_timestamp) = parse_rfc3339_utc(&args.to) else {
        ui::print_generic_error(HISTORY_ERR_INVALID_TO_TIMESTAMP);

        return ExitCode::from(EXIT_CODE_INVALID_INPUT);
    };

    if from_timestamp > to_timestamp {
        ui::print_generic_error(HISTORY_ERR_INVALID_RANGE_FROM_GT_TO);

        return ExitCode::from(EXIT_CODE_INVALID_INPUT);
    }

    let project_root = match resolve_project_root(&args.cwd) {
        Ok(path) => path,
        Err(error) => {
            let error_message = format_err_with_reason(HISTORY_ERR_RESOLVE_CWD, &error);

            ui::print_generic_error(&error_message);

            return ExitCode::FAILURE;
        }
    };
    let ledger_path = resolve_history_ledger_path(&project_root);

    let project_filter = match resolve_project_filter(args.project.as_ref()) {
        Ok(project_filter) => project_filter,
        Err(exit_code) => return exit_code,
    };

    let filters = HistoryQueryFilters {
        from: from_timestamp,
        to: to_timestamp,
        package: args.package.clone(),
        version: args.version.clone(),
        project: project_filter.clone(),
        package_manager: args.package_manager.clone(),
    };

    let query_history_events_result = query_history_events(&ledger_path, &filters);

    let events = match query_history_events_result {
        Ok(events) => events,
        Err(error) => {
            ui::print_generic_error(&error);

            return ExitCode::FAILURE;
        }
    };

    let query = HistoryQuery {
        from: args.from.clone(),
        to: args.to.clone(),
        package: args.package.clone(),
        version: args.version.clone(),
        project: project_filter,
        package_manager: args.package_manager.clone(),
    };

    let history_run_mode = classify_history_run_mode(args.package.as_ref());

    match history_run_mode {
        HistoryRunMode::Package => {
            let output = HistoryPackageModeOutput {
                query,
                found: !events.is_empty(),
                matches: events,
            };
            let render_package_mode_params = RenderPackageModeParams {
                output: &output,
                format: &args.format,
                quiet: args.quiet,
            };

            history_output::render_package_mode(render_package_mode_params);

            ExitCode::SUCCESS
        }
        HistoryRunMode::Range => {
            let packages = summarize_unique_packages(&events);
            let output = HistoryRangeModeOutput {
                query,
                summary: HistoryRangeSummary {
                    events: events.len(),
                    projects: count_unique_projects(&events),
                    unique_packages: packages.len(),
                },
                packages,
            };
            let render_range_mode_params = RenderRangeModeParams {
                output: &output,
                format: &args.format,
                quiet: args.quiet,
            };

            history_output::render_range_mode(render_range_mode_params);

            ExitCode::SUCCESS
        }
    }
}
