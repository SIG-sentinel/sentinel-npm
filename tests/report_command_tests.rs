use sentinel::types::PrintReportSubmissionParams;
use sentinel::ui::report_submission_lines;

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
