#[path = "report_output_github.rs"]
mod report_output_github;
#[path = "report_output_json.rs"]
mod report_output_json;
#[path = "report_output_junit.rs"]
mod report_output_junit;
#[path = "report_output_text.rs"]
mod report_output_text;

use crate::constants::{
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_COMPROMISED, OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_REGISTRY_UNAVAILABLE, OUTPUT_INSTALL_BLOCKED_HINT_COMPROMISED,
    OUTPUT_INSTALL_BLOCKED_NEXT_HEADER, OUTPUT_INSTALL_BLOCKED_TEMPLATE,
    OUTPUT_NEXT_ACTION_CI_DEFAULT, OUTPUT_NEXT_ACTION_COMPROMISED, OUTPUT_NEXT_ACTION_GITHUB_CI,
    OUTPUT_NEXT_ACTION_INSTALL_DEFAULT, OUTPUT_NEXT_ACTION_STRICT_CI,
    OUTPUT_NEXT_ACTION_STRICT_INSTALL, OUTPUT_REASON_MISSING_FROM_LOCKFILE,
    OUTPUT_REASON_NO_INTEGRITY_FIELD, OUTPUT_REASON_REGISTRY_OFFLINE,
    OUTPUT_REASON_REGISTRY_TIMEOUT, OUTPUT_STATUS_ALL_CLEAN, OUTPUT_STATUS_BLOCKED,
    OUTPUT_STATUS_WARNINGS, OUTPUT_SUMMARY_LINE_TEMPLATE, OUTPUT_SYMBOL_WARNING, UI_LABEL_NEXT,
    UI_LABEL_TIP, render_template, render_with_error,
};
use crate::types::{
    OutputFormat, PrintReportParams, Summary, UnverifiableReason, Verdict, VerifyResult,
};
use colored::Colorize;

fn unverifiable_reason_text(reason: &UnverifiableReason) -> &'static str {
    match reason {
        UnverifiableReason::NoIntegrityField => OUTPUT_REASON_NO_INTEGRITY_FIELD,
        UnverifiableReason::RegistryOffline => OUTPUT_REASON_REGISTRY_OFFLINE,
        UnverifiableReason::RegistryTimeout => OUTPUT_REASON_REGISTRY_TIMEOUT,
        UnverifiableReason::MissingFromLockfile => OUTPUT_REASON_MISSING_FROM_LOCKFILE,
    }
}

fn print_install_blocked_header() {
    println!();
    println!(
        "{}",
        render_with_error(OUTPUT_INSTALL_BLOCKED_TEMPLATE, &[])
    );
}

fn print_unverifiable_entry(
    verify_result: &VerifyResult,
    unverifiable_reason: &UnverifiableReason,
) {
    println!(
        "  {} {} — {}",
        OUTPUT_SYMBOL_WARNING.yellow().bold(),
        verify_result.package.to_string().bold(),
        unverifiable_reason_text(unverifiable_reason).dimmed()
    );
}

pub fn print_report(params: PrintReportParams<'_>) {
    let PrintReportParams {
        report,
        output_format,
    } = params;

    match output_format {
        OutputFormat::Text => report_output_text::print_text(report),
        OutputFormat::Json => report_output_json::print_json(report),
        OutputFormat::Github => report_output_github::print_github_annotations(report),
        OutputFormat::Junit => report_output_junit::print_junit(report),
    }
}

pub(super) fn print_summary_line(summary: &Summary) {
    let has_compromised_packages = summary.compromised > 0;
    let has_unverifiable_packages = summary.unverifiable > 0;

    let status_text = match &summary.exit_code {
        0 if !has_compromised_packages && !has_unverifiable_packages => {
            OUTPUT_STATUS_ALL_CLEAN.green().bold()
        }
        0 => OUTPUT_STATUS_WARNINGS.yellow().bold(),
        _ => OUTPUT_STATUS_BLOCKED.red().bold(),
    };

    let unverifiable_count_text = match summary.unverifiable {
        0 => summary.unverifiable.to_string().normal(),
        _ => summary.unverifiable.to_string().yellow(),
    };
    let compromised_count_text = match summary.compromised {
        0 => summary.compromised.to_string().normal(),
        _ => summary.compromised.to_string().red().bold(),
    };
    let summary_line = render_template(
        OUTPUT_SUMMARY_LINE_TEMPLATE,
        &[
            status_text.to_string(),
            summary.total.to_string(),
            summary.clean.to_string().green().to_string(),
            unverifiable_count_text.to_string(),
            compromised_count_text.to_string(),
        ],
    );
    println!("{summary_line}");
    println!();

    print_user_next_steps(summary);
}

fn print_user_next_steps(summary: &Summary) {
    match (summary.compromised > 0, summary.unverifiable > 0) {
        (true, _) => {
            println!(
                "{}",
                render_template(
                    OUTPUT_NEXT_ACTION_COMPROMISED,
                    &[UI_LABEL_NEXT.red().bold().to_string()]
                )
            );
            println!(
                "{}",
                render_template(
                    OUTPUT_NEXT_ACTION_GITHUB_CI,
                    &[UI_LABEL_TIP.yellow().bold().to_string()]
                )
            );
            println!();
        }
        (false, true) => {
            println!(
                "{}",
                render_template(
                    OUTPUT_NEXT_ACTION_STRICT_CI,
                    &[UI_LABEL_TIP.yellow().bold().to_string()]
                )
            );
            println!(
                "{}",
                render_template(
                    OUTPUT_NEXT_ACTION_STRICT_INSTALL,
                    &[UI_LABEL_TIP.yellow().bold().to_string()]
                )
            );
            println!();
        }
        (false, false) => {
            println!(
                "{}",
                render_template(
                    OUTPUT_NEXT_ACTION_INSTALL_DEFAULT,
                    &[UI_LABEL_TIP.green().bold().to_string()]
                )
            );
            println!(
                "{}",
                render_template(
                    OUTPUT_NEXT_ACTION_CI_DEFAULT,
                    &[UI_LABEL_TIP.green().bold().to_string()]
                )
            );
            println!();
        }
    }
}

pub fn print_install_blocked(results: &[VerifyResult]) {
    print_install_blocked_header();
    report_output_text::print_compromised_results(results);
    println!("{OUTPUT_INSTALL_BLOCKED_NEXT_HEADER}");
    println!();
    println!("  {}", OUTPUT_INSTALL_BLOCKED_GUIDANCE_COMPROMISED.dimmed());
    for result in results {
        if matches!(result.verdict, Verdict::Compromised { .. }) {
            println!(
                "  {}",
                render_template(
                    OUTPUT_INSTALL_BLOCKED_HINT_COMPROMISED,
                    &[
                        result.package.name.cyan().to_string(),
                        result.package.version.cyan().to_string(),
                    ]
                )
                .dimmed()
            );
        }
    }
    println!();
}

pub fn print_install_blocked_unverifiable(results: &[VerifyResult]) {
    print_install_blocked_header();
    for result in results {
        if let Verdict::Unverifiable { reason } = &result.verdict {
            print_unverifiable_entry(result, reason);
        }
    }
    println!("{OUTPUT_INSTALL_BLOCKED_NEXT_HEADER}");
    println!();

    let has_no_integrity = results.iter().any(|result| {
        matches!(
            result.verdict,
            Verdict::Unverifiable {
                reason: UnverifiableReason::NoIntegrityField
            }
        )
    });
    let has_registry_unavailable = results.iter().any(|result| {
        matches!(
            result.verdict,
            Verdict::Unverifiable {
                reason: UnverifiableReason::RegistryOffline | UnverifiableReason::RegistryTimeout
            }
        )
    });
    let has_not_in_lockfile = results.iter().any(|result| {
        matches!(
            result.verdict,
            Verdict::Unverifiable {
                reason: UnverifiableReason::MissingFromLockfile
            }
        )
    });

    if has_no_integrity {
        println!(
            "  {}",
            OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY.dimmed()
        );
        println!();
    }
    if has_registry_unavailable {
        println!(
            "  {}",
            OUTPUT_INSTALL_BLOCKED_GUIDANCE_REGISTRY_UNAVAILABLE.dimmed()
        );
        println!();
    }
    if has_not_in_lockfile {
        println!(
            "  {}",
            OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE.dimmed()
        );
        println!();
    }
}
