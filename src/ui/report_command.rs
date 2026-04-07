use colored::Colorize;

use crate::constants::{
    CLI_PREFIX_SENTINEL, REPORT_MSG_EVIDENCE_TEMPLATE, REPORT_MSG_GITHUB_ADVISORY,
    REPORT_MSG_INTRO, REPORT_MSG_NPM_SECURITY, REPORT_MSG_OUTRO, UI_REPORT_HEADER_SYMBOL_TEMPLATE,
    render_template,
};
use crate::types::PrintReportSubmissionParams;

pub fn print_report_submission(params: PrintReportSubmissionParams<'_>) {
    let PrintReportSubmissionParams {
        package_name,
        evidence,
    } = params;

    println!(
        "{}",
        render_template(
            UI_REPORT_HEADER_SYMBOL_TEMPLATE,
            &[
                CLI_PREFIX_SENTINEL.green().bold().to_string(),
                package_name.bold().to_string(),
            ]
        )
    );
    println!("{REPORT_MSG_INTRO}");
    println!("{REPORT_MSG_NPM_SECURITY}");
    println!("{REPORT_MSG_GITHUB_ADVISORY}");

    if let Some(evidence_text) = evidence {
        println!(
            "{}",
            render_template(REPORT_MSG_EVIDENCE_TEMPLATE, &[evidence_text.to_string()])
        );
    }

    println!("{REPORT_MSG_OUTRO}");
}
