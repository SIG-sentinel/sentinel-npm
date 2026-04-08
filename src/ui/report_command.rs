use colored::Colorize;

use crate::constants::{
    CLI_PREFIX_SENTINEL, REPORT_MSG_EVIDENCE_TEMPLATE, REPORT_MSG_GITHUB_ADVISORY,
    REPORT_MSG_INTRO, REPORT_MSG_NPM_SECURITY, REPORT_MSG_OUTRO, REPORT_MSG_REASON_TEMPLATE,
    UI_REPORT_HEADER_SYMBOL_TEMPLATE, render_template,
};
use crate::types::PrintReportSubmissionParams;

fn report_submission_lines(params: PrintReportSubmissionParams<'_>) -> Vec<String> {
    let PrintReportSubmissionParams {
        package_name,
        reason,
        evidence,
    } = params;

    let mut lines = vec![render_template(
        UI_REPORT_HEADER_SYMBOL_TEMPLATE,
        &[
            CLI_PREFIX_SENTINEL.green().bold().to_string(),
            package_name.bold().to_string(),
        ],
    )];

    lines.push(REPORT_MSG_INTRO.to_string());
    lines.push(REPORT_MSG_NPM_SECURITY.to_string());
    lines.push(REPORT_MSG_GITHUB_ADVISORY.to_string());
    lines.push(render_template(
        REPORT_MSG_REASON_TEMPLATE,
        &[reason.to_string()],
    ));

    if let Some(evidence_text) = evidence {
        lines.push(render_template(
            REPORT_MSG_EVIDENCE_TEMPLATE,
            &[evidence_text.to_string()],
        ));
    }

    lines.push(REPORT_MSG_OUTRO.to_string());

    lines
}

pub fn print_report_submission(params: PrintReportSubmissionParams<'_>) {
    for line in report_submission_lines(params) {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::report_submission_lines;
    use crate::types::PrintReportSubmissionParams;

    #[test]
    fn test_report_submission_lines_include_reason_and_evidence() {
        let lines = report_submission_lines(PrintReportSubmissionParams {
            package_name: "left-pad@1.3.0",
            reason: "suspicious lifecycle script",
            evidence: Some("https://example.test/evidence"),
        });

        let rendered = lines.join("\n");

        assert!(rendered.contains("suspicious lifecycle script"));
        assert!(rendered.contains("https://example.test/evidence"));
    }
}
