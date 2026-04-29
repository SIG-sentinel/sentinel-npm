use crate::constants::{
    OUTPUT_XML_ERROR, OUTPUT_XML_HEADER, OUTPUT_XML_TESTCASE_CLEAN, OUTPUT_XML_TESTCASE_CLOSE,
    OUTPUT_XML_TESTCASE_COMPROMISED, OUTPUT_XML_TESTCASE_UNVERIFIABLE, OUTPUT_XML_TESTSUITE,
    OUTPUT_XML_TESTSUITE_CLOSE, OUTPUT_XML_TESTSUITES, OUTPUT_XML_TESTSUITES_CLOSE,
    UI_JUNIT_SYSTEM_OUT_TEMPLATE, render_template,
};
use crate::types::{Report, Verdict};

fn print_rendered_template(template: &str, template_args: &[String]) {
    println!("{}", render_template(template, template_args));
}

fn print_testcase_with_name(template: &str, escaped_package_name: &str) {
    let testcase_template_args = vec![escaped_package_name.to_string()];

    print_rendered_template(template, &testcase_template_args);
}

fn print_unverifiable_testcase(escaped_package_name: &str, reason: &str, detail: &str) {
    print_testcase_with_name(OUTPUT_XML_TESTCASE_UNVERIFIABLE, escaped_package_name);

    let system_out_template_args = vec![reason.to_string(), xml_escape(detail)];

    print_rendered_template(UI_JUNIT_SYSTEM_OUT_TEMPLATE, &system_out_template_args);
    println!("{OUTPUT_XML_TESTCASE_CLOSE}");
}

fn print_compromised_testcase(escaped_package_name: &str, detail: &str) {
    print_testcase_with_name(OUTPUT_XML_TESTCASE_COMPROMISED, escaped_package_name);

    let error_template_args = vec![xml_escape(detail)];

    print_rendered_template(OUTPUT_XML_ERROR, &error_template_args);
    println!("{OUTPUT_XML_TESTCASE_CLOSE}");
}

pub(super) fn print_junit(report: &Report) {
    let errors = report.summary.compromised;
    let provenance_warnings = report.summary.provenance_summary.provenance_missing_count;
    let blocking_unverifiable = report
        .summary
        .unverifiable
        .saturating_sub(provenance_warnings);
    let total = report.summary.total;
    let testsuites_template_args = vec![
        total.to_string(),
        errors.to_string(),
        provenance_warnings.to_string(),
        blocking_unverifiable.to_string(),
        provenance_warnings.to_string(),
    ];

    let testsuite_template_args = vec![total.to_string(), errors.to_string()];

    println!("{OUTPUT_XML_HEADER}");
    print_rendered_template(OUTPUT_XML_TESTSUITES, &testsuites_template_args);
    print_rendered_template(OUTPUT_XML_TESTSUITE, &testsuite_template_args);

    for result in &report.results {
        let escaped_package_name = xml_escape(&result.package.to_string());

        match &result.verdict {
            Verdict::Clean => {
                print_testcase_with_name(OUTPUT_XML_TESTCASE_CLEAN, &escaped_package_name);
            }
            Verdict::Unverifiable { reason } => {
                let reason_text = format!("{reason:?}");

                print_unverifiable_testcase(&escaped_package_name, &reason_text, &result.detail);
            }
            Verdict::Compromised { .. } => {
                print_compromised_testcase(&escaped_package_name, &result.detail);
            }
        }
    }

    println!("{OUTPUT_XML_TESTSUITE_CLOSE}");
    println!("{OUTPUT_XML_TESTSUITES_CLOSE}");
}

fn xml_escape(raw_text: &str) -> String {
    raw_text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
