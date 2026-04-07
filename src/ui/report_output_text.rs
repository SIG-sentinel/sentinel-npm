use crate::constants::{OUTPUT_INSTALL_DETAIL_LINES, OUTPUT_LABEL_COMPROMISED};
use crate::types::{Report, Verdict, VerdictFilter};
use colored::Colorize;

use super::{print_summary_line, print_unverifiable_entry};

pub(super) fn print_text(report: &Report) {
    println!();

    let compromised = report.get_compromised();
    let unverifiable = report.get_unverifiable();

    for compromised_result in &compromised {
        if let Verdict::Compromised { .. } = &compromised_result.verdict {
            println!(
                "  {} {}",
                OUTPUT_LABEL_COMPROMISED.red().bold().on_black(),
                compromised_result.package.to_string().bold()
            );
            println!("  {}", compromised_result.detail.dimmed());
            println!();
        }
    }

    for unverifiable_result in &unverifiable {
        if let Verdict::Unverifiable { reason } = &unverifiable_result.verdict {
            print_unverifiable_entry(unverifiable_result, reason);
        }
    }

    if !unverifiable.is_empty() {
        println!();
    }

    print_summary_line(&report.summary);
}

pub(super) fn print_compromised_results(results: &[crate::types::VerifyResult]) {
    for result in results {
        if matches!(result.verdict, Verdict::Compromised { .. }) {
            println!(
                "  {} {}",
                OUTPUT_LABEL_COMPROMISED.red().bold(),
                result.package.to_string().bold()
            );

            let detail_preview = result
                .detail
                .lines()
                .take(OUTPUT_INSTALL_DETAIL_LINES)
                .map(|line| format!("    {}", line.dimmed()))
                .collect::<Vec<_>>()
                .join("\n");

            if !detail_preview.is_empty() {
                println!("{detail_preview}");
            }

            println!();
        }
    }
}
