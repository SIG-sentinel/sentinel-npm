use super::report::Summary;

#[derive(Debug, Clone, Copy)]
pub struct SummaryPolicyInput {
    pub total: u32,
    pub clean: u32,
    pub compromised: u32,
    pub unverifiable: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallBlockReason {
    Compromised,
    Unverifiable,
}

#[derive(Debug, Clone, Copy)]
pub struct InstallPolicyInput {
    pub compromised_count: usize,
    pub unverifiable_count: usize,
    pub allow_scripts: bool,
    pub no_scripts: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct InstallPolicyDecision {
    pub block_reason: Option<InstallBlockReason>,
    pub ignore_scripts: bool,
}

pub trait SecurityPolicy {
    fn check_summary(&self, input: SummaryPolicyInput) -> Summary;

    fn install_decision(&self, input: InstallPolicyInput) -> InstallPolicyDecision;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultSecurityPolicy;
