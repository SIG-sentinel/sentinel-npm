use crate::constants::{
    OUTPUT_INSTALL_DETAIL_LINES, OUTPUT_LABEL_COMPROMISED, OUTPUT_LABEL_DIRECT,
    OUTPUT_LABEL_PARENT_HINT, OUTPUT_LABEL_TRANSITIVE,
    OUTPUT_PROVENANCE_MISSING_SUPPRESSED_TEMPLATE, render_template,
};
use crate::types::{PrintSummaryLineParams, PrintUnverifiableEntryParams, Report, VerdictFilter};
use colored::Colorize;

use super::{print_summary_line, print_unverifiable_entry};

const PROVENANCE_MISSING_TOP_N: usize = 10;

fn format_dependency_label(is_direct_dependency: bool) -> String {
    let mut dependency_label = OUTPUT_LABEL_TRANSITIVE.dimmed().to_string();
    if is_direct_dependency {
        dependency_label = OUTPUT_LABEL_DIRECT.yellow().to_string();
    }

    dependency_label
}

fn print_compromised_header(package_name: &str, dependency_label: &str, highlight: bool) {
    let mut compromised_label = OUTPUT_LABEL_COMPROMISED.red().bold().to_string();
    if highlight {
        compromised_label = OUTPUT_LABEL_COMPROMISED.red().bold().on_black().to_string();
    }

    println!(
        "  {} {} [{}]",
        compromised_label,
        package_name.bold(),
        dependency_label,
    );
}

fn print_parent_hint_line(direct_parent: Option<&String>) {
    let Some(parent) = direct_parent else {
        return;
    };

    println!(
        "     {} {}",
        OUTPUT_LABEL_PARENT_HINT.dimmed(),
        parent.dimmed()
    );
}

fn print_detail_lines(detail: &str) {
    for line in detail.lines() {
        println!("     {}", line.dimmed());
    }
}

fn build_detail_preview(detail: &str) -> String {
    detail
        .lines()
        .take(OUTPUT_INSTALL_DETAIL_LINES)
        .map(|line| format!("    {}", line.dimmed()))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn print_text(report: &Report) {
    println!();

    let compromised = report.get_compromised();
    let unverifiable = report.get_unverifiable();

    for compromised_result in &compromised {
        if !compromised_result.is_compromised() {
            continue;
        }

        let dependency_label = format_dependency_label(compromised_result.is_direct);
        let package_name = compromised_result.package.to_string();

        print_compromised_header(&package_name, &dependency_label, true);
        print_parent_hint_line(compromised_result.direct_parent.as_ref());
        print_detail_lines(&compromised_result.detail);
        println!();
    }

    let (unverifiable_to_display, suppressed_provenance_missing) =
        split_unverifiable_for_text_output(&unverifiable);

    for unverifiable_result in &unverifiable_to_display {
        let Some(reason) = unverifiable_result.unverifiable_reason() else {
            continue;
        };

        let print_unverifiable_entry_params = PrintUnverifiableEntryParams {
            verify_result: unverifiable_result,
            unverifiable_reason: reason,
        };

        print_unverifiable_entry(print_unverifiable_entry_params);
    }

    let has_suppressed_provenance_missing = suppressed_provenance_missing > 0;

    if has_suppressed_provenance_missing {
        let suppressed_template_args = vec![
            suppressed_provenance_missing.to_string(),
            PROVENANCE_MISSING_TOP_N.to_string(),
        ];

        println!(
            "{}",
            render_template(
                OUTPUT_PROVENANCE_MISSING_SUPPRESSED_TEMPLATE,
                &suppressed_template_args,
            )
            .dimmed()
        );
    }

    if !unverifiable.is_empty() {
        println!();
    }

    let print_summary_line_params = PrintSummaryLineParams {
        summary: &report.summary,
        results: &report.results,
    };
    print_summary_line(print_summary_line_params);
}

fn split_unverifiable_for_text_output<'a>(
    unverifiable_results: &[&'a crate::types::VerifyResult],
) -> (Vec<&'a crate::types::VerifyResult>, usize) {
    let mut display = Vec::new();
    let mut provenance_missing = Vec::new();

    for result in unverifiable_results {
        if result.is_provenance_missing() {
            provenance_missing.push(*result);

            continue;
        }

        display.push(*result);
    }

    let shown_missing = provenance_missing.len().min(PROVENANCE_MISSING_TOP_N);
    let total_provenance_missing = provenance_missing.len();

    display.extend(provenance_missing.into_iter().take(shown_missing));

    let suppressed_missing = total_provenance_missing.saturating_sub(shown_missing);

    (display, suppressed_missing)
}

#[cfg(test)]
#[path = "../../tests/internal/report_output_text_internal_tests.rs"]
mod tests;

pub(super) fn print_compromised_results(results: &[crate::types::VerifyResult]) {
    for result in results {
        if !result.is_compromised() {
            continue;
        }

        let dependency_label = format_dependency_label(result.is_direct);
        let package_name = result.package.to_string();

        print_compromised_header(&package_name, &dependency_label, false);
        print_parent_hint_line(result.direct_parent.as_ref());

        let detail_preview = build_detail_preview(&result.detail);

        if !detail_preview.is_empty() {
            println!("{detail_preview}");
        }

        println!();
    }
}
