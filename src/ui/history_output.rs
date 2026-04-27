use crate::constants::{
    HISTORY_TEXT_FOUND_NO, HISTORY_TEXT_FOUND_YES, HISTORY_TEXT_LABEL_EVENTS,
    HISTORY_TEXT_LABEL_FOUND, HISTORY_TEXT_LABEL_PACKAGE, HISTORY_TEXT_LABEL_PACKAGES,
    HISTORY_TEXT_LABEL_PROJECTS, HISTORY_TEXT_LABEL_RANGE, HISTORY_TEXT_LABEL_UNIQUE,
    HISTORY_TEXT_UNKNOWN_PACKAGE,
};
use crate::types::{
    HistoryOutputFormat, PrintPackageModeJsonOutputParams, PrintRangeModeJsonOutputParams,
    RenderPackageModeParams, RenderRangeModeParams, ShouldPrintHistoryOutputParams,
};

fn should_print_output(params: ShouldPrintHistoryOutputParams<'_>) -> bool {
    let ShouldPrintHistoryOutputParams { format, quiet } = params;
    let is_text_output = matches!(format, HistoryOutputFormat::Text);

    !quiet || !is_text_output
}

fn print_package_mode_json_output(params: PrintPackageModeJsonOutputParams<'_>) {
    let PrintPackageModeJsonOutputParams { output } = params;

    if let Ok(serialized_output_json) = serde_json::to_string_pretty(output) {
        println!("{serialized_output_json}");
    }
}

fn print_range_mode_json_output(params: PrintRangeModeJsonOutputParams<'_>) {
    let PrintRangeModeJsonOutputParams { output } = params;

    if let Ok(serialized_output_json) = serde_json::to_string_pretty(output) {
        println!("{serialized_output_json}");
    }
}

pub fn render_package_mode(params: RenderPackageModeParams<'_>) {
    let RenderPackageModeParams {
        output,
        format,
        quiet,
    } = params;
    let should_print_output_params = ShouldPrintHistoryOutputParams { format, quiet };
    let should_print = should_print_output(should_print_output_params);

    if !should_print {
        return;
    }

    match format {
        HistoryOutputFormat::Text => {
            let query = &output.query;
            let package = query
                .package
                .as_deref()
                .unwrap_or(HISTORY_TEXT_UNKNOWN_PACKAGE);
            let mut found_text = HISTORY_TEXT_FOUND_NO;
            if output.found {
                found_text = HISTORY_TEXT_FOUND_YES;
            }

            println!("{HISTORY_TEXT_LABEL_PACKAGE} {package}");
            println!(
                "{HISTORY_TEXT_LABEL_RANGE}   {} .. {}",
                query.from, query.to
            );
            println!("{HISTORY_TEXT_LABEL_FOUND}   {found_text}");
            println!("{HISTORY_TEXT_LABEL_EVENTS}  {}", output.matches.len());

            if !output.matches.is_empty() {
                println!();
            }

            for event in &output.matches {
                let occurred_at = &event.occurred_at;
                let project_root = &event.project_root;
                let package_manager = &event.package_manager;
                let command = &event.command;
                let package = &event.package;
                let package_name = &package.name;
                let package_version = &package.version;

                println!(
                    "{occurred_at}  {project_root}  {package_manager}  {command}  {package_name}@{package_version}",
                );
            }
        }
        HistoryOutputFormat::Json => {
            let print_package_mode_json_output_params = PrintPackageModeJsonOutputParams { output };
            print_package_mode_json_output(print_package_mode_json_output_params);
        }
    }
}

pub fn render_range_mode(params: RenderRangeModeParams<'_>) {
    let RenderRangeModeParams {
        output,
        format,
        quiet,
    } = params;
    let should_print_output_params = ShouldPrintHistoryOutputParams { format, quiet };
    let should_print = should_print_output(should_print_output_params);

    if !should_print {
        return;
    }

    match format {
        HistoryOutputFormat::Text => {
            let query = &output.query;
            let summary = &output.summary;

            println!(
                "{HISTORY_TEXT_LABEL_RANGE}    {} .. {}",
                query.from, query.to
            );
            println!("{HISTORY_TEXT_LABEL_PROJECTS} {}", summary.projects);
            println!("{HISTORY_TEXT_LABEL_EVENTS}   {}", summary.events);
            println!(
                "{HISTORY_TEXT_LABEL_PACKAGES} {} {HISTORY_TEXT_LABEL_UNIQUE}",
                summary.unique_packages,
            );

            if !output.packages.is_empty() {
                println!();
            }

            for package in &output.packages {
                println!("{package}");
            }
        }
        HistoryOutputFormat::Json => {
            let print_range_mode_json_output_params = PrintRangeModeJsonOutputParams { output };
            print_range_mode_json_output(print_range_mode_json_output_params);
        }
    }
}
