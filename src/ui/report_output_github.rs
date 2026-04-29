use crate::constants::{
    GITHUB_PROVENANCE_MISSING_PREVIEW_COUNT, OUTPUT_GITHUB_ERROR_FORMAT, OUTPUT_GITHUB_ERROR_TITLE,
    OUTPUT_GITHUB_LOCKFILE_REF, OUTPUT_GITHUB_PROVENANCE_MISSING_TITLE,
    OUTPUT_GITHUB_SUMMARY_CLEAN_TEMPLATE, OUTPUT_GITHUB_SUMMARY_COMPROMISED_TEMPLATE,
    OUTPUT_GITHUB_SUMMARY_UNVERIFIABLE_TEMPLATE, OUTPUT_GITHUB_WARNING_TITLE,
    UI_GITHUB_WARNING_FORMAT, render_template,
};
use crate::types::{Report, Verdict};

const ZERO_FINDINGS: u32 = 0;

fn print_rendered_template(template: &str, template_args: &[String]) {
    println!("{}", render_template(template, template_args));
}

fn print_compromised_annotation(report: &Report, index: usize) {
    let result = &report.results[index];
    let github_error_template_args = vec![
        OUTPUT_GITHUB_ERROR_TITLE.to_string(),
        OUTPUT_GITHUB_LOCKFILE_REF.to_string(),
        result.package.to_string(),
        result.detail.lines().next().unwrap_or("").to_string(),
    ];

    print_rendered_template(OUTPUT_GITHUB_ERROR_FORMAT, &github_error_template_args);
}

fn print_unverifiable_annotation(report: &Report, index: usize) {
    let result = &report.results[index];
    let Verdict::Unverifiable { reason } = &result.verdict else {
        return;
    };

    let github_warning_template_args = vec![
        OUTPUT_GITHUB_WARNING_TITLE.to_string(),
        OUTPUT_GITHUB_LOCKFILE_REF.to_string(),
        result.package.to_string(),
        format!("{reason:?}"),
    ];

    print_rendered_template(UI_GITHUB_WARNING_FORMAT, &github_warning_template_args);
}

fn print_provenance_missing_summary(packages: &[String]) {
    let total = packages.len();

    if total == 0 {
        return;
    }

    let preview = packages
        .iter()
        .take(GITHUB_PROVENANCE_MISSING_PREVIEW_COUNT)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    let suffix = if total > GITHUB_PROVENANCE_MISSING_PREVIEW_COUNT {
        format!(
            " (+{} more)",
            total - GITHUB_PROVENANCE_MISSING_PREVIEW_COUNT
        )
    } else {
        String::new()
    };

    println!(
        "::notice title={OUTPUT_GITHUB_PROVENANCE_MISSING_TITLE},file={OUTPUT_GITHUB_LOCKFILE_REF}::{total} package(s) lack provenance — {preview}{suffix}",
    );
}

fn resolve_summary_template_and_count(report: &Report) -> (&'static str, u32) {
    let has_compromised_findings = report.summary.compromised > ZERO_FINDINGS;
    let has_unverifiable_findings = report.summary.unverifiable > ZERO_FINDINGS;

    match (has_compromised_findings, has_unverifiable_findings) {
        (true, _) => (
            OUTPUT_GITHUB_SUMMARY_COMPROMISED_TEMPLATE,
            report.summary.compromised,
        ),
        (false, true) => (
            OUTPUT_GITHUB_SUMMARY_UNVERIFIABLE_TEMPLATE,
            report.summary.unverifiable,
        ),
        (false, false) => (OUTPUT_GITHUB_SUMMARY_CLEAN_TEMPLATE, report.summary.clean),
    }
}

fn print_summary(report: &Report) {
    let (summary_template, summary_count) = resolve_summary_template_and_count(report);
    let summary_template_args = vec![summary_count.to_string()];

    print_rendered_template(summary_template, &summary_template_args);
}

pub(super) fn print_github_annotations(report: &Report) {
    let mut provenance_missing_packages: Vec<String> = Vec::new();

    for (index, result) in report.results.iter().enumerate() {
        if matches!(result.verdict, Verdict::Compromised { .. }) {
            print_compromised_annotation(report, index);
        }
    }

    for (index, result) in report.results.iter().enumerate() {
        let Verdict::Unverifiable { .. } = &result.verdict else {
            continue;
        };

        if result.is_provenance_missing() {
            provenance_missing_packages.push(result.package.to_string());

            continue;
        }

        print_unverifiable_annotation(report, index);
    }

    print_provenance_missing_summary(&provenance_missing_packages);

    print_summary(report);
}
