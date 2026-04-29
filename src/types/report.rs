use serde::{Deserialize, Serialize};

use super::policy::{DefaultSecurityPolicy, SecurityPolicy, SummaryPolicyInput};
use super::verification::VerifyResult;

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub sentinel_version: String,
    pub timestamp: String,
    pub mode: RunMode,
    pub summary: Summary,
    pub results: Vec<VerifyResult>,
    pub cycles: Vec<Vec<String>>,
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
    #[serde(default)]
    pub provenance_summary: ProvenanceSummary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ProvenanceSummary {
    pub trusted_count: u32,
    pub warning_count: u32,
    pub inconsistent_count: u32,
    pub provenance_missing_count: u32,
    pub provenance_missing_shown: u32,
    pub provenance_missing_suppressed: u32,
    pub trust_coverage: f64,
    pub provenance_availability: f64,
}

impl Default for ProvenanceSummary {
    fn default() -> Self {
        Self {
            trusted_count: 0,
            warning_count: 0,
            inconsistent_count: 0,
            provenance_missing_count: 0,
            provenance_missing_shown: 0,
            provenance_missing_suppressed: 0,
            trust_coverage: 0.0,
            provenance_availability: 0.0,
        }
    }
}

impl Summary {
    pub fn from_results(results: &[VerifyResult]) -> Self {
        let total = u32::try_from(results.len()).unwrap_or(u32::MAX);
        let clean = u32::try_from(results.iter().filter(|result| result.is_clean()).count())
            .unwrap_or(u32::MAX);
        let compromised = u32::try_from(
            results
                .iter()
                .filter(|result| result.is_compromised())
                .count(),
        )
        .unwrap_or(u32::MAX);
        let unverifiable = u32::try_from(
            results
                .iter()
                .filter(|result| result.is_unverifiable())
                .count(),
        )
        .unwrap_or(u32::MAX);
        let blocking_unverifiable = u32::try_from(
            results
                .iter()
                .filter(|result| result.is_blocking_unverifiable())
                .count(),
        )
        .unwrap_or(u32::MAX);

        let summary_input = SummaryPolicyInput {
            total,
            clean,
            compromised,
            unverifiable,
            blocking_unverifiable,
        };

        let mut summary = DefaultSecurityPolicy.check_summary(summary_input);

        let trusted_count = u32::try_from(
            results
                .iter()
                .filter(|result| {
                    result.is_clean() && result.evidence.provenance_subject_digest.is_some()
                })
                .count(),
        )
        .unwrap_or(u32::MAX);
        let inconsistent_count = u32::try_from(
            results
                .iter()
                .filter(|result| result.is_provenance_inconsistent())
                .count(),
        )
        .unwrap_or(u32::MAX);
        let provenance_missing_count = u32::try_from(
            results
                .iter()
                .filter(|result| result.is_provenance_missing())
                .count(),
        )
        .unwrap_or(u32::MAX);
        let warning_count = provenance_missing_count;
        let packages_with_provenance = trusted_count.saturating_add(inconsistent_count);
        let mut trust_coverage = 0.0;
        if packages_with_provenance > 0 {
            trust_coverage = f64::from(trusted_count) / f64::from(packages_with_provenance);
        }

        let mut provenance_availability = 0.0;
        if total > 0 {
            provenance_availability = f64::from(packages_with_provenance) / f64::from(total);
        }

        summary.provenance_summary = ProvenanceSummary {
            trusted_count,
            warning_count,
            inconsistent_count,
            provenance_missing_count,
            provenance_missing_shown: provenance_missing_count,
            provenance_missing_suppressed: 0,
            trust_coverage,
            provenance_availability,
        };

        summary
    }
}

impl Report {
    pub fn from_results(
        mode: RunMode,
        results: Vec<VerifyResult>,
        cycles: Vec<Vec<String>>,
    ) -> Self {
        Self {
            sentinel_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            mode,
            summary: Summary::from_results(&results),
            results,
            cycles,
        }
    }
}
