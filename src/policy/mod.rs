pub use crate::types::{
    DefaultSecurityPolicy, InstallBlockReason, InstallPolicyDecision, InstallPolicyInput,
    SecurityPolicy, SummaryPolicyInput,
};
use crate::types::{ProvenanceSummary, Summary};

const ZERO_SUMMARY_COUNT: u32 = 0;
const ZERO_INSTALL_COUNT: usize = 0;
const EXIT_CODE_SUCCESS: i32 = 0;
const EXIT_CODE_FAILURE: i32 = 1;

impl SecurityPolicy for DefaultSecurityPolicy {
    fn check_summary(&self, input: SummaryPolicyInput) -> Summary {
        let SummaryPolicyInput {
            total,
            clean,
            compromised,
            unverifiable,
            blocking_unverifiable,
        } = input;

        let has_compromised_packages = compromised > ZERO_SUMMARY_COUNT;
        let has_blocking_unverifiable_packages = blocking_unverifiable > ZERO_SUMMARY_COUNT;
        let exit_code = match (has_compromised_packages, has_blocking_unverifiable_packages) {
            (true, _) | (_, true) => EXIT_CODE_FAILURE,
            _ => EXIT_CODE_SUCCESS,
        };

        Summary {
            total,
            clean,
            unverifiable,
            compromised,
            exit_code,
            provenance_summary: ProvenanceSummary::default(),
        }
    }

    fn install_decision(&self, input: InstallPolicyInput) -> InstallPolicyDecision {
        let InstallPolicyInput {
            compromised_count,
            unverifiable_count,
            allow_scripts,
            post_verify,
        } = input;

        let has_compromised_packages = compromised_count > ZERO_INSTALL_COUNT;
        let has_unverifiable_packages = unverifiable_count > ZERO_INSTALL_COUNT;
        let block_reason = match (has_compromised_packages, has_unverifiable_packages) {
            (true, _) => Some(InstallBlockReason::Compromised),
            (_, true) => Some(InstallBlockReason::Unverifiable),
            _ => None,
        };

        let scripts_not_allowed = !allow_scripts;
        let post_verify_without_scripts = post_verify && scripts_not_allowed;
        let ignore_scripts = scripts_not_allowed || post_verify_without_scripts;

        InstallPolicyDecision {
            block_reason,
            ignore_scripts,
        }
    }
}
