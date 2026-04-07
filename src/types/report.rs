use serde::{Deserialize, Serialize};

use super::policy::{DefaultSecurityPolicy, SecurityPolicy, SummaryPolicyInput};
use super::verification::{Verdict, VerifyResult};

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub sentinel_version: String,
    pub timestamp: String,
    pub mode: RunMode,
    pub summary: Summary,
    pub results: Vec<VerifyResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunMode {
    Check,
    Install,
    Ci,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Summary {
    pub total: u32,
    pub clean: u32,
    pub unverifiable: u32,
    pub compromised: u32,
    pub exit_code: i32,
}

impl Summary {
    pub fn from_results(results: &[VerifyResult]) -> Self {
        let total = results.len() as u32;
        let clean = results
            .iter()
            .filter(|result| result.verdict == Verdict::Clean)
            .count() as u32;
        let compromised = results
            .iter()
            .filter(|result| matches!(result.verdict, Verdict::Compromised { .. }))
            .count() as u32;
        let unverifiable = results
            .iter()
            .filter(|result| matches!(result.verdict, Verdict::Unverifiable { .. }))
            .count() as u32;

        let summary_input = SummaryPolicyInput {
            total,
            clean,
            compromised,
            unverifiable,
        };

        DefaultSecurityPolicy.check_summary(summary_input)
    }
}

impl Report {
    pub fn from_results(mode: RunMode, results: Vec<VerifyResult>) -> Self {
        Self {
            sentinel_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            mode,
            summary: Summary::from_results(&results),
            results,
        }
    }
}
