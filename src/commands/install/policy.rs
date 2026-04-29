use crate::output::{print_install_blocked, print_install_blocked_unverifiable};
use crate::policy::{DefaultSecurityPolicy, InstallPolicyDecision, SecurityPolicy};
use crate::types::{
    BlockedVerifyResults, InstallBlockReason, InstallPolicyInput, PrintBlockReasonResultsParams,
    ResolveInstallPolicyParams, VerifyResult,
};

pub(super) fn collect_blocked_verify_results(results: &[VerifyResult]) -> BlockedVerifyResults {
    let compromised = results
        .iter()
        .filter(|result| result.is_compromised())
        .cloned()
        .collect();
    let unverifiable = results
        .iter()
        .filter(|result| result.is_blocking_unverifiable())
        .cloned()
        .collect();

    BlockedVerifyResults {
        compromised,
        unverifiable,
    }
}

pub(super) fn print_block_reason_results(params: PrintBlockReasonResultsParams<'_>) {
    let PrintBlockReasonResultsParams {
        block_reason,
        blocked,
    } = params;

    match block_reason {
        InstallBlockReason::Compromised => print_install_blocked(&blocked.compromised),
        InstallBlockReason::Unverifiable => {
            print_install_blocked_unverifiable(&blocked.unverifiable);
        }
    }
}

pub(super) fn resolve_install_block_reason(
    blocked: &BlockedVerifyResults,
) -> Option<InstallBlockReason> {
    let has_compromised_results = !blocked.compromised.is_empty();
    let has_unverifiable_results = !blocked.unverifiable.is_empty();

    match (has_compromised_results, has_unverifiable_results) {
        (true, _) => Some(InstallBlockReason::Compromised),
        (false, true) => Some(InstallBlockReason::Unverifiable),
        (false, false) => None,
    }
}

pub(super) fn resolve_install_policy(params: ResolveInstallPolicyParams) -> InstallPolicyDecision {
    let ResolveInstallPolicyParams {
        compromised_count,
        unverifiable_count,
        allow_scripts,
        post_verify,
    } = params;

    let install_policy_input = InstallPolicyInput {
        compromised_count,
        unverifiable_count,
        allow_scripts,
        post_verify,
    };

    DefaultSecurityPolicy.install_decision(install_policy_input)
}
