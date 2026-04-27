use crate::constants::{
    OUTPUT_GITHUB_ERROR_FORMAT, OUTPUT_GITHUB_ERROR_TITLE, OUTPUT_GITHUB_LOCKFILE_REF,
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
    for (index, result) in report.results.iter().enumerate() {
        let should_print_compromised = matches!(result.verdict, Verdict::Compromised { .. });

        if should_print_compromised {
            print_compromised_annotation(report, index);

            continue;
        }

        let should_print_unverifiable = matches!(result.verdict, Verdict::Unverifiable { .. });

        if should_print_unverifiable {
            print_unverifiable_annotation(report, index);
        }
    }

    print_summary(report);
}
