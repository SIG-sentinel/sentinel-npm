use crate::constants::{
    OUTPUT_GITHUB_ERROR_FORMAT, OUTPUT_GITHUB_ERROR_TITLE, OUTPUT_GITHUB_LOCKFILE_REF,
    OUTPUT_GITHUB_SUMMARY_CLEAN_TEMPLATE, OUTPUT_GITHUB_SUMMARY_COMPROMISED_TEMPLATE,
    OUTPUT_GITHUB_SUMMARY_UNVERIFIABLE_TEMPLATE, OUTPUT_GITHUB_WARNING_TITLE,
    UI_GITHUB_WARNING_FORMAT, render_template,
};
use crate::types::{Report, Verdict};

pub(super) fn print_github_annotations(report: &Report) {
    for result in &report.results {
        match &result.verdict {
            Verdict::Compromised { .. } => {
                println!(
                    "{}",
                    render_template(
                        OUTPUT_GITHUB_ERROR_FORMAT,
                        &[
                            OUTPUT_GITHUB_ERROR_TITLE.to_string(),
                            OUTPUT_GITHUB_LOCKFILE_REF.to_string(),
                            result.package.to_string(),
                            result.detail.lines().next().unwrap_or("").to_string(),
                        ],
                    )
                );
            }
            Verdict::Unverifiable { reason } => {
                println!(
                    "{}",
                    render_template(
                        UI_GITHUB_WARNING_FORMAT,
                        &[
                            OUTPUT_GITHUB_WARNING_TITLE.to_string(),
                            OUTPUT_GITHUB_LOCKFILE_REF.to_string(),
                            result.package.to_string(),
                            format!("{:?}", reason),
                        ]
                    )
                );
            }
            Verdict::Clean => {}
        }
    }

    match (report.summary.compromised, report.summary.unverifiable) {
        (compromised_count, _) if compromised_count > 0 => println!(
            "{}",
            render_template(
                OUTPUT_GITHUB_SUMMARY_COMPROMISED_TEMPLATE,
                &[report.summary.compromised.to_string()]
            )
        ),
        (_, unverifiable_count) if unverifiable_count > 0 => println!(
            "{}",
            render_template(
                OUTPUT_GITHUB_SUMMARY_UNVERIFIABLE_TEMPLATE,
                &[report.summary.unverifiable.to_string()]
            )
        ),
        _ => println!(
            "{}",
            render_template(
                OUTPUT_GITHUB_SUMMARY_CLEAN_TEMPLATE,
                &[report.summary.clean.to_string()]
            )
        ),
    }
}
