#[path = "report_output_github.rs"]
mod report_output_github;
#[path = "report_output_json.rs"]
mod report_output_json;
#[path = "report_output_junit.rs"]
mod report_output_junit;
#[path = "report_output_text.rs"]
mod report_output_text;

use crate::constants::{
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_COMPROMISED,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_LEGACY_SHA1_LOCKFILE,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY_DIRECT,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY_TRANSITIVE,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE_DIRECT,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE_TRANSITIVE,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_PROVENANCE_INCONSISTENT,
    OUTPUT_INSTALL_BLOCKED_GUIDANCE_REGISTRY_UNAVAILABLE, OUTPUT_INSTALL_BLOCKED_HINT_COMPROMISED,
    OUTPUT_INSTALL_BLOCKED_NEXT_HEADER, OUTPUT_INSTALL_BLOCKED_TEMPLATE, OUTPUT_LABEL_DIRECT,
    OUTPUT_LABEL_PARENT_HINT, OUTPUT_LABEL_TRANSITIVE, OUTPUT_NEXT_ACTION_CI_DEFAULT,
    OUTPUT_NEXT_ACTION_COMPROMISED, OUTPUT_NEXT_ACTION_GITHUB_CI,
    OUTPUT_NEXT_ACTION_INSTALL_DEFAULT, OUTPUT_NEXT_ACTION_LEGACY_SHA1_LOCKFILE,
    OUTPUT_NEXT_ACTION_LOCKFILE_STALE_DIRECT, OUTPUT_NEXT_ACTION_LOCKFILE_STALE_TRANSITIVE,
    OUTPUT_NEXT_ACTION_OLD_PACKAGE_DIRECT, OUTPUT_NEXT_ACTION_OLD_PACKAGE_TRANSITIVE,
    OUTPUT_NEXT_ACTION_PROVENANCE_INCONSISTENT, OUTPUT_NEXT_ACTION_PROVENANCE_MISSING,
    OUTPUT_NEXT_ACTION_REGISTRY_UNAVAILABLE, OUTPUT_NEXT_ACTION_STRICT_CI,
    OUTPUT_PROVENANCE_SUMMARY_TEMPLATE, OUTPUT_REASON_LEGACY_SHA1_LOCKFILE,
    OUTPUT_REASON_MISSING_FROM_LOCKFILE, OUTPUT_REASON_NO_INTEGRITY_FIELD,
    OUTPUT_REASON_PROVENANCE_INCONSISTENT, OUTPUT_REASON_PROVENANCE_MISSING,
    OUTPUT_REASON_REGISTRY_OFFLINE, OUTPUT_REASON_REGISTRY_TIMEOUT,
    OUTPUT_REASON_TARBALL_TOO_LARGE, OUTPUT_STATUS_ALL_CLEAN, OUTPUT_STATUS_BLOCKED,
    OUTPUT_STATUS_WARNINGS, OUTPUT_SUMMARY_LINE_TEMPLATE, OUTPUT_SYMBOL_WARNING, UI_LABEL_NEXT,
    UI_LABEL_TIP, render_template, render_with_error,
};
use crate::types::{
    InstallBlockedMissingFromLockfileFlags, InstallBlockedNoIntegrityFlags,
    InstallBlockedUnverifiableGuidanceFlags, OutputFormat, ParentScopedGuidanceMode,
    PrintDimmedGuidanceLineParams, PrintParentScopedGuidanceParams, PrintReportParams,
    PrintSummaryLineParams, PrintTemplateLineParams, PrintUnverifiableEntryParams,
    PrintUserNextStepsParams, UnverifiableIntegrityFlags, UnverifiableLockfileFlags,
    UnverifiableNextActionFlags, UnverifiableProvenanceFlags, UnverifiableReason, VerifyResult,
};

use colored::Colorize;
use std::collections::BTreeMap;

const ZERO_COUNT: u32 = 0;
const ZERO_EXIT_CODE: i32 = 0;
const OUTPUT_TRANSITIVE_BLOCKERS_HEADER: &str = "  transitive blockers by direct dependency:";

fn print_template_line(params: PrintTemplateLineParams<'_>) {
    let PrintTemplateLineParams {
        template,
        template_args,
    } = params;

    println!("{}", render_template(template, template_args));
}

fn print_dimmed_guidance_line(params: PrintDimmedGuidanceLineParams<'_>) {
    let PrintDimmedGuidanceLineParams { guidance } = params;

    println!("  {}", guidance.dimmed());
}

fn print_blank_line() {
    println!();
}

fn format_percentage(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

fn print_next_action_lines(template_args: &[String], templates: &[&'static str]) {
    for template in templates {
        let template_line_params = PrintTemplateLineParams {
            template,
            template_args,
        };

        print_template_line(template_line_params);
    }

    print_blank_line();
}

fn resolve_unverifiable_next_action_flags(results: &[VerifyResult]) -> UnverifiableNextActionFlags {
    let has_no_integrity_direct = results
        .iter()
        .any(VerifyResult::is_direct_no_integrity_field);

    let has_no_integrity_transitive = results
        .iter()
        .any(VerifyResult::is_transitive_no_integrity_field);

    let has_lockfile_stale_direct = results
        .iter()
        .any(VerifyResult::is_direct_missing_from_lockfile);

    let has_lockfile_stale_transitive = results
        .iter()
        .any(VerifyResult::is_transitive_missing_from_lockfile);

    let has_registry_unavailable = results.iter().any(VerifyResult::is_registry_unavailable);
    let has_legacy_sha1_lockfile = results.iter().any(VerifyResult::is_legacy_sha1_lockfile);
    let has_provenance_missing = results.iter().any(VerifyResult::is_provenance_missing);
    let has_provenance_inconsistent = results.iter().any(VerifyResult::is_provenance_inconsistent);

    UnverifiableNextActionFlags {
        integrity: UnverifiableIntegrityFlags {
            has_direct: has_no_integrity_direct,
            has_transitive: has_no_integrity_transitive,
        },
        lockfile: UnverifiableLockfileFlags {
            has_direct_stale: has_lockfile_stale_direct,
            has_transitive_stale: has_lockfile_stale_transitive,
            has_legacy_sha1: has_legacy_sha1_lockfile,
        },
        provenance: UnverifiableProvenanceFlags {
            has_missing: has_provenance_missing,
            has_inconsistent: has_provenance_inconsistent,
        },
        has_registry_unavailable,
    }
}

fn resolve_unverifiable_next_action_templates(
    flags: UnverifiableNextActionFlags,
) -> Vec<&'static str> {
    let UnverifiableNextActionFlags {
        integrity,
        lockfile,
        provenance,
        has_registry_unavailable,
    } = flags;

    let template_candidates = [
        (integrity.has_direct, OUTPUT_NEXT_ACTION_OLD_PACKAGE_DIRECT),
        (
            integrity.has_transitive,
            OUTPUT_NEXT_ACTION_OLD_PACKAGE_TRANSITIVE,
        ),
        (
            lockfile.has_direct_stale,
            OUTPUT_NEXT_ACTION_LOCKFILE_STALE_DIRECT,
        ),
        (
            lockfile.has_transitive_stale,
            OUTPUT_NEXT_ACTION_LOCKFILE_STALE_TRANSITIVE,
        ),
        (
            has_registry_unavailable,
            OUTPUT_NEXT_ACTION_REGISTRY_UNAVAILABLE,
        ),
        (
            lockfile.has_legacy_sha1,
            OUTPUT_NEXT_ACTION_LEGACY_SHA1_LOCKFILE,
        ),
        (
            provenance.has_missing,
            OUTPUT_NEXT_ACTION_PROVENANCE_MISSING,
        ),
        (
            provenance.has_inconsistent,
            OUTPUT_NEXT_ACTION_PROVENANCE_INCONSISTENT,
        ),
    ];

    let mut templates: Vec<&'static str> = template_candidates
        .into_iter()
        .filter_map(|(is_enabled, template)| is_enabled.then_some(template))
        .collect();

    if templates.is_empty() {
        templates.push(OUTPUT_NEXT_ACTION_STRICT_CI);
    }

    templates
}

fn resolve_install_blocked_unverifiable_guidance_flags(
    results: &[VerifyResult],
) -> InstallBlockedUnverifiableGuidanceFlags {
    let has_no_integrity_direct = results
        .iter()
        .any(VerifyResult::is_direct_no_integrity_field);

    let has_no_integrity_transitive = results
        .iter()
        .any(VerifyResult::is_transitive_no_integrity_field);

    let has_no_integrity = results.iter().any(VerifyResult::is_no_integrity_field);
    let has_registry_unavailable = results.iter().any(VerifyResult::is_registry_unavailable);

    let has_not_in_lockfile_direct = results
        .iter()
        .any(VerifyResult::is_direct_missing_from_lockfile);

    let has_not_in_lockfile_transitive = results
        .iter()
        .any(VerifyResult::is_transitive_missing_from_lockfile);

    let has_not_in_lockfile = results.iter().any(VerifyResult::is_missing_from_lockfile);
    let has_legacy_sha1_lockfile = results.iter().any(VerifyResult::is_legacy_sha1_lockfile);
    let has_provenance_inconsistent = results.iter().any(VerifyResult::is_provenance_inconsistent);

    InstallBlockedUnverifiableGuidanceFlags {
        no_integrity: InstallBlockedNoIntegrityFlags {
            has_direct: has_no_integrity_direct,
            has_transitive: has_no_integrity_transitive,
            has_any: has_no_integrity,
        },
        missing_from_lockfile: InstallBlockedMissingFromLockfileFlags {
            has_direct: has_not_in_lockfile_direct,
            has_transitive: has_not_in_lockfile_transitive,
            has_any: has_not_in_lockfile,
        },
        has_registry_unavailable,
        has_legacy_sha1_lockfile,
        has_provenance_inconsistent,
    }
}

fn classify_parent_scoped_guidance(
    params: PrintParentScopedGuidanceParams<'_>,
) -> ParentScopedGuidanceMode<'_> {
    let PrintParentScopedGuidanceParams {
        has_group,
        has_direct,
        has_transitive,
        direct_guidance,
        transitive_guidance,
        fallback_guidance,
    } = params;

    match (has_group, has_direct, has_transitive) {
        (false, _, _) => ParentScopedGuidanceMode::Skip,
        (true, true, false) => ParentScopedGuidanceMode::DirectOnly { direct_guidance },
        (true, false, true) => ParentScopedGuidanceMode::TransitiveOnly {
            transitive_guidance,
        },
        (true, true, true) => ParentScopedGuidanceMode::DirectAndTransitive {
            direct_guidance,
            transitive_guidance,
        },
        (true, false, false) => ParentScopedGuidanceMode::FallbackOnly { fallback_guidance },
    }
}

fn print_parent_scoped_guidance(params: PrintParentScopedGuidanceParams<'_>) {
    let parent_scoped_guidance_mode = classify_parent_scoped_guidance(params);

    match parent_scoped_guidance_mode {
        ParentScopedGuidanceMode::Skip => {}
        ParentScopedGuidanceMode::DirectOnly { direct_guidance } => {
            let dimmed_guidance_line_params = PrintDimmedGuidanceLineParams {
                guidance: direct_guidance,
            };

            print_dimmed_guidance_line(dimmed_guidance_line_params);
            print_blank_line();
            print_blank_line();
        }
        ParentScopedGuidanceMode::TransitiveOnly {
            transitive_guidance,
        } => {
            let dimmed_guidance_line_params = PrintDimmedGuidanceLineParams {
                guidance: transitive_guidance,
            };

            print_dimmed_guidance_line(dimmed_guidance_line_params);
            print_blank_line();
        }
        ParentScopedGuidanceMode::DirectAndTransitive {
            direct_guidance,
            transitive_guidance,
        } => {
            let dimmed_guidance_line_params = PrintDimmedGuidanceLineParams {
                guidance: direct_guidance,
            };

            print_dimmed_guidance_line(dimmed_guidance_line_params);
            print_blank_line();

            let dimmed_guidance_line_params = PrintDimmedGuidanceLineParams {
                guidance: transitive_guidance,
            };

            print_dimmed_guidance_line(dimmed_guidance_line_params);
            print_blank_line();
        }
        ParentScopedGuidanceMode::FallbackOnly { fallback_guidance } => {
            let dimmed_guidance_line_params = PrintDimmedGuidanceLineParams {
                guidance: fallback_guidance,
            };

            print_dimmed_guidance_line(dimmed_guidance_line_params);
            print_blank_line();
        }
    }
}

fn print_optional_guidance(has_guidance: bool, guidance: &'static str) {
    if !has_guidance {
        return;
    }

    let dimmed_guidance_line_params = PrintDimmedGuidanceLineParams { guidance };

    print_dimmed_guidance_line(dimmed_guidance_line_params);
    print_blank_line();
}

fn unverifiable_reason_text(reason: UnverifiableReason) -> &'static str {
    match reason {
        UnverifiableReason::NoIntegrityField => OUTPUT_REASON_NO_INTEGRITY_FIELD,
        UnverifiableReason::LegacySha1Lockfile => OUTPUT_REASON_LEGACY_SHA1_LOCKFILE,
        UnverifiableReason::RegistryOffline => OUTPUT_REASON_REGISTRY_OFFLINE,
        UnverifiableReason::RegistryTimeout => OUTPUT_REASON_REGISTRY_TIMEOUT,
        UnverifiableReason::MissingFromLockfile => OUTPUT_REASON_MISSING_FROM_LOCKFILE,
        UnverifiableReason::TarballTooLarge => OUTPUT_REASON_TARBALL_TOO_LARGE,
        UnverifiableReason::ProvenanceMissing => OUTPUT_REASON_PROVENANCE_MISSING,
        UnverifiableReason::ProvenanceInconsistent => OUTPUT_REASON_PROVENANCE_INCONSISTENT,
    }
}

fn print_install_blocked_header() {
    let install_blocked_header_template_args: Vec<String> = Vec::new();

    print_blank_line();
    println!(
        "{}",
        render_with_error(
            OUTPUT_INSTALL_BLOCKED_TEMPLATE,
            &install_blocked_header_template_args,
        )
    );
}

fn print_unverifiable_entry(params: PrintUnverifiableEntryParams<'_>) {
    let PrintUnverifiableEntryParams {
        verify_result,
        unverifiable_reason,
    } = params;

    let mut dep_label = OUTPUT_LABEL_TRANSITIVE.dimmed().to_string();

    if verify_result.is_direct {
        dep_label = OUTPUT_LABEL_DIRECT.yellow().to_string();
    }

    println!(
        "  {} {} — {} [{}]",
        OUTPUT_SYMBOL_WARNING.yellow().bold(),
        verify_result.package.to_string().bold(),
        unverifiable_reason_text(*unverifiable_reason).dimmed(),
        dep_label,
    );

    print_parent_hint_line(verify_result.direct_parent.as_deref());
}

fn print_parent_hint_line(parent: Option<&str>) {
    if let Some(parent_name) = parent {
        println!(
            "     {} {}",
            OUTPUT_LABEL_PARENT_HINT.dimmed(),
            parent_name.dimmed(),
        );
    }
}

fn group_transitive_blockers_by_parent(results: &[VerifyResult]) -> BTreeMap<String, Vec<String>> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for result in results {
        let is_transitive = !result.is_direct;
        let Some(parent) = result.direct_parent.as_ref() else {
            continue;
        };

        if !is_transitive {
            continue;
        }

        grouped
            .entry(parent.clone())
            .or_default()
            .push(result.package.to_string());
    }

    for packages in grouped.values_mut() {
        packages.sort();
        packages.dedup();
    }

    grouped
}

fn print_transitive_blockers_by_parent(grouped: &BTreeMap<String, Vec<String>>) {
    if grouped.is_empty() {
        return;
    }

    println!("{OUTPUT_TRANSITIVE_BLOCKERS_HEADER}");

    for (parent, packages) in grouped {
        println!(
            "    {} {}",
            OUTPUT_LABEL_PARENT_HINT.dimmed(),
            parent.dimmed()
        );
        for package in packages {
            println!("      - {}", package.dimmed());
        }
    }

    print_blank_line();
}

fn print_compromised_blocked_hint(verify_result: &VerifyResult) {
    let blocked_hint_template_args = vec![
        verify_result.package.name.cyan().to_string(),
        verify_result.package.version.cyan().to_string(),
    ];

    println!(
        "  {}",
        render_template(
            OUTPUT_INSTALL_BLOCKED_HINT_COMPROMISED,
            &blocked_hint_template_args,
        )
        .dimmed()
    );
}

fn print_compromised_blocked_hints(results: &[VerifyResult]) {
    for verify_result in results.iter().filter(|result| result.is_compromised()) {
        print_compromised_blocked_hint(verify_result);
    }
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

pub(super) fn print_summary_line(params: PrintSummaryLineParams<'_>) {
    let PrintSummaryLineParams { summary, results } = params;

    let has_compromised_packages = summary.compromised > ZERO_COUNT;
    let has_unverifiable_packages = summary.unverifiable > ZERO_COUNT;
    let has_clean_exit_without_issues = !has_compromised_packages && !has_unverifiable_packages;

    let provenance_warning_count = summary.provenance_summary.provenance_missing_count;
    let blocking_unverifiable_count = summary
        .unverifiable
        .saturating_sub(provenance_warning_count);

    let status_text = match summary.exit_code {
        ZERO_EXIT_CODE if has_clean_exit_without_issues => OUTPUT_STATUS_ALL_CLEAN.green().bold(),
        ZERO_EXIT_CODE => OUTPUT_STATUS_WARNINGS.yellow().bold(),
        _ => OUTPUT_STATUS_BLOCKED.red().bold(),
    };

    let blocking_unverifiable_count_text = match blocking_unverifiable_count {
        ZERO_COUNT => blocking_unverifiable_count.to_string().normal(),
        _ => blocking_unverifiable_count.to_string().yellow(),
    };
    let provenance_warning_count_text = match provenance_warning_count {
        ZERO_COUNT => provenance_warning_count.to_string().normal(),
        _ => provenance_warning_count.to_string().yellow(),
    };
    let compromised_count_text = match summary.compromised {
        ZERO_COUNT => summary.compromised.to_string().normal(),
        _ => summary.compromised.to_string().red().bold(),
    };
    let summary_line_template_args = vec![
        status_text.to_string(),
        summary.total.to_string(),
        summary.clean.to_string().green().to_string(),
        blocking_unverifiable_count_text.to_string(),
        provenance_warning_count_text.to_string(),
        compromised_count_text.to_string(),
    ];
    let summary_line = render_template(OUTPUT_SUMMARY_LINE_TEMPLATE, &summary_line_template_args);

    println!("{summary_line}");

    let provenance_summary = summary.provenance_summary;
    let coverage_percent = format_percentage(provenance_summary.trust_coverage);
    let availability_percent = format_percentage(provenance_summary.provenance_availability);
    let provenance_summary_template_args = vec![
        provenance_summary.trusted_count.to_string(),
        provenance_summary.warning_count.to_string(),
        provenance_summary.inconsistent_count.to_string(),
        coverage_percent,
        availability_percent,
    ];

    println!(
        "{}",
        render_template(
            OUTPUT_PROVENANCE_SUMMARY_TEMPLATE,
            &provenance_summary_template_args,
        )
    );
    println!();

    let user_next_steps_params = PrintUserNextStepsParams { summary, results };

    print_user_next_steps(user_next_steps_params);
}

#[allow(clippy::too_many_lines)]
fn print_user_next_steps(params: PrintUserNextStepsParams<'_>) {
    let PrintUserNextStepsParams { summary, results } = params;
    let tip = UI_LABEL_TIP.yellow().bold().to_string();
    let next = UI_LABEL_NEXT.red().bold().to_string();

    match (
        summary.compromised > ZERO_COUNT,
        summary.unverifiable > ZERO_COUNT,
    ) {
        (true, _) => {
            let compromised_next_action_template_args = vec![next];
            let compromised_next_action_templates = [OUTPUT_NEXT_ACTION_COMPROMISED];
            print_next_action_lines(
                &compromised_next_action_template_args,
                &compromised_next_action_templates,
            );

            let github_ci_next_action_template_args = vec![tip.clone()];
            let github_ci_next_action_templates = [OUTPUT_NEXT_ACTION_GITHUB_CI];

            print_next_action_lines(
                &github_ci_next_action_template_args,
                &github_ci_next_action_templates,
            );
        }
        (false, true) => {
            let tip_template_args = vec![tip.clone()];
            let unverifiable_next_action_flags = resolve_unverifiable_next_action_flags(results);
            let unverifiable_next_action_templates =
                resolve_unverifiable_next_action_templates(unverifiable_next_action_flags);

            print_next_action_lines(&tip_template_args, &unverifiable_next_action_templates);
        }
        (false, false) => {
            let install_default_template_args = vec![tip.clone()];
            let default_next_action_templates = [
                OUTPUT_NEXT_ACTION_INSTALL_DEFAULT,
                OUTPUT_NEXT_ACTION_CI_DEFAULT,
            ];

            print_next_action_lines(
                &install_default_template_args,
                &default_next_action_templates,
            );
        }
    }
}

pub fn print_install_blocked(results: &[VerifyResult]) {
    print_install_blocked_header();
    report_output_text::print_compromised_results(results);
    println!("{OUTPUT_INSTALL_BLOCKED_NEXT_HEADER}");
    print_blank_line();

    let dimmed_guidance_line_params = PrintDimmedGuidanceLineParams {
        guidance: OUTPUT_INSTALL_BLOCKED_GUIDANCE_COMPROMISED,
    };

    print_dimmed_guidance_line(dimmed_guidance_line_params);
    print_compromised_blocked_hints(results);
    print_blank_line();
}

#[allow(clippy::too_many_lines)]
pub fn print_install_blocked_unverifiable(results: &[VerifyResult]) {
    print_install_blocked_header();
    let unverifiable_entries = results.iter().filter_map(|verify_result| {
        verify_result
            .unverifiable_reason()
            .map(|unverifiable_reason| (verify_result, unverifiable_reason))
    });

    for (verify_result, unverifiable_reason) in unverifiable_entries {
        let unverifiable_entry_params = PrintUnverifiableEntryParams {
            verify_result,
            unverifiable_reason,
        };

        print_unverifiable_entry(unverifiable_entry_params);
    }

    let transitive_blockers_by_parent = group_transitive_blockers_by_parent(results);

    print_transitive_blockers_by_parent(&transitive_blockers_by_parent);
    println!("{OUTPUT_INSTALL_BLOCKED_NEXT_HEADER}");
    print_blank_line();

    let install_blocked_unverifiable_guidance_flags =
        resolve_install_blocked_unverifiable_guidance_flags(results);
    let InstallBlockedUnverifiableGuidanceFlags {
        no_integrity,
        missing_from_lockfile,
        has_registry_unavailable,
        has_legacy_sha1_lockfile,
        has_provenance_inconsistent,
    } = install_blocked_unverifiable_guidance_flags;

    let parent_scoped_guidance_params = PrintParentScopedGuidanceParams {
        has_group: no_integrity.has_any,
        has_direct: no_integrity.has_direct,
        has_transitive: no_integrity.has_transitive,
        direct_guidance: OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY_DIRECT,
        transitive_guidance: OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY_TRANSITIVE,
        fallback_guidance: OUTPUT_INSTALL_BLOCKED_GUIDANCE_NO_INTEGRITY,
    };

    print_parent_scoped_guidance(parent_scoped_guidance_params);

    print_optional_guidance(
        has_registry_unavailable,
        OUTPUT_INSTALL_BLOCKED_GUIDANCE_REGISTRY_UNAVAILABLE,
    );

    let parent_scoped_guidance_params = PrintParentScopedGuidanceParams {
        has_group: missing_from_lockfile.has_any,
        has_direct: missing_from_lockfile.has_direct,
        has_transitive: missing_from_lockfile.has_transitive,
        direct_guidance: OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE_DIRECT,
        transitive_guidance: OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE_TRANSITIVE,
        fallback_guidance: OUTPUT_INSTALL_BLOCKED_GUIDANCE_NOT_IN_LOCKFILE,
    };

    print_parent_scoped_guidance(parent_scoped_guidance_params);
    print_optional_guidance(
        has_legacy_sha1_lockfile,
        OUTPUT_INSTALL_BLOCKED_GUIDANCE_LEGACY_SHA1_LOCKFILE,
    );
    print_optional_guidance(
        has_provenance_inconsistent,
        OUTPUT_INSTALL_BLOCKED_GUIDANCE_PROVENANCE_INCONSISTENT,
    );
}
