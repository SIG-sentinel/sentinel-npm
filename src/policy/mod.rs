use crate::types::Summary;
pub use crate::types::{
    DefaultSecurityPolicy, InstallBlockReason, InstallPolicyDecision, InstallPolicyInput,
    SecurityPolicy, SummaryPolicyInput,
};

impl SecurityPolicy for DefaultSecurityPolicy {
    fn check_summary(&self, input: SummaryPolicyInput) -> Summary {
        let SummaryPolicyInput {
            total,
            clean,
            compromised,
            unverifiable,
        } = input;

        let exit_code = match (compromised > 0, unverifiable > 0) {
            (true, _) | (_, true) => 1,
            _ => 0,
        };

        Summary {
            total,
            clean,
            unverifiable,
            compromised,
            exit_code,
        }
    }

    fn install_decision(&self, input: InstallPolicyInput) -> InstallPolicyDecision {
        let block_reason = match (input.compromised_count > 0, input.unverifiable_count > 0) {
            (true, _) => Some(InstallBlockReason::Compromised),
            (_, true) => Some(InstallBlockReason::Unverifiable),
            _ => None,
        };

        let ignore_scripts = match (input.no_scripts, input.allow_scripts) {
            (true, _) => true,
            (false, false) => input.unverifiable_count > 0,
            _ => false,
        };

        InstallPolicyDecision {
            block_reason,
            ignore_scripts,
        }
    }
}
