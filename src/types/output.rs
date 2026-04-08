use super::{Report, UnverifiableReason, VerifyResult};

#[derive(Debug, Clone, clap::ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Github,
    Junit,
}

pub enum OutputPrefix {
    Warning,
}

impl OutputPrefix {
    pub fn colored(&self) -> String {
        use crate::constants::CLI_PREFIX_WARNING;
        use colored::Colorize;

        match self {
            OutputPrefix::Warning => CLI_PREFIX_WARNING.yellow().bold().to_string(),
        }
    }
}

pub struct PrintReportParams<'a> {
    pub report: &'a Report,
    pub output_format: &'a OutputFormat,
}

pub struct PrintUnverifiableEntryParams<'a> {
    pub verify_result: &'a VerifyResult,
    pub unverifiable_reason: &'a UnverifiableReason,
}

pub struct PrintReportSubmissionParams<'a> {
    pub package_name: &'a str,
    pub reason: &'a str,
    pub evidence: Option<&'a str>,
}
