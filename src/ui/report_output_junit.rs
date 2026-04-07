use crate::constants::{
    OUTPUT_XML_ERROR, OUTPUT_XML_HEADER, OUTPUT_XML_TESTCASE_CLEAN, OUTPUT_XML_TESTCASE_CLOSE,
    OUTPUT_XML_TESTCASE_COMPROMISED, OUTPUT_XML_TESTCASE_UNVERIFIABLE, OUTPUT_XML_TESTSUITE,
    OUTPUT_XML_TESTSUITE_CLOSE, OUTPUT_XML_TESTSUITES, OUTPUT_XML_TESTSUITES_CLOSE,
    UI_JUNIT_SYSTEM_OUT_TEMPLATE, render_template,
};
use crate::types::{Report, Verdict};

pub(super) fn print_junit(report: &Report) {
    let errors = report.summary.compromised;
    let warnings = report.summary.unverifiable;
    let total = report.summary.total;

    println!("{}", OUTPUT_XML_HEADER);
    println!(
        "{}",
        render_template(
            OUTPUT_XML_TESTSUITES,
            &[total.to_string(), errors.to_string(), warnings.to_string()]
        )
    );
    println!(
        "{}",
        render_template(
            OUTPUT_XML_TESTSUITE,
            &[total.to_string(), errors.to_string()]
        )
    );

    for result in &report.results {
        let escaped_package_name = xml_escape(&result.package.to_string());

        match &result.verdict {
            Verdict::Clean => {
                println!(
                    "{}",
                    render_template(OUTPUT_XML_TESTCASE_CLEAN, &[escaped_package_name])
                );
            }
            Verdict::Unverifiable { reason } => {
                println!(
                    "{}",
                    render_template(OUTPUT_XML_TESTCASE_UNVERIFIABLE, &[escaped_package_name])
                );
                println!(
                    "{}",
                    render_template(
                        UI_JUNIT_SYSTEM_OUT_TEMPLATE,
                        &[format!("{:?}", reason), xml_escape(&result.detail)]
                    )
                );
                println!("{}", OUTPUT_XML_TESTCASE_CLOSE);
            }
            Verdict::Compromised { .. } => {
                println!(
                    "{}",
                    render_template(OUTPUT_XML_TESTCASE_COMPROMISED, &[escaped_package_name])
                );
                println!(
                    "{}",
                    render_template(OUTPUT_XML_ERROR, &[xml_escape(&result.detail)])
                );
                println!("{}", OUTPUT_XML_TESTCASE_CLOSE);
            }
        }
    }

    println!("{}", OUTPUT_XML_TESTSUITE_CLOSE);
    println!("{}", OUTPUT_XML_TESTSUITES_CLOSE);
}

fn xml_escape(raw_text: &str) -> String {
    raw_text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
