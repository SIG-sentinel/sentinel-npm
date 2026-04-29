use super::package::PackageRef;
use super::{Report, Summary, UnverifiableReason, VerifyResult};

#[derive(Debug, Clone, clap::ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Github,
    Junit,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum ArtifactStore {
    Memory,
    Spool,
    Auto,
}

impl ArtifactStore {
    pub fn as_env_value(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Spool => "spool",
            Self::Auto => "auto",
        }
    }

    pub fn from_env_value(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "memory" => Some(Self::Memory),
            "spool" => Some(Self::Spool),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/internal/output_types_tests.rs"]
mod tests;

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

#[derive(Clone, Copy)]
pub struct PrintReportParams<'a> {
    pub report: &'a Report,
    pub output_format: &'a OutputFormat,
}

#[derive(Clone, Copy)]
pub struct PrintUnverifiableEntryParams<'a> {
    pub verify_result: &'a VerifyResult,
    pub unverifiable_reason: &'a UnverifiableReason,
}

#[derive(Clone, Copy)]
pub struct PrintUserNextStepsParams<'a> {
    pub summary: &'a Summary,
    pub results: &'a [VerifyResult],
}

#[derive(Clone, Copy)]
pub struct PrintSummaryLineParams<'a> {
    pub summary: &'a Summary,
    pub results: &'a [VerifyResult],
}

#[derive(Clone, Copy)]
pub struct UnverifiableIntegrityFlags {
    pub has_direct: bool,
    pub has_transitive: bool,
}

#[derive(Clone, Copy)]
pub struct UnverifiableLockfileFlags {
    pub has_direct_stale: bool,
    pub has_transitive_stale: bool,
    pub has_legacy_sha1: bool,
}

#[derive(Clone, Copy)]
pub struct UnverifiableProvenanceFlags {
    pub has_missing: bool,
    pub has_inconsistent: bool,
}

#[derive(Clone, Copy)]
pub struct UnverifiableNextActionFlags {
    pub integrity: UnverifiableIntegrityFlags,
    pub lockfile: UnverifiableLockfileFlags,
    pub provenance: UnverifiableProvenanceFlags,
    pub has_registry_unavailable: bool,
}

#[derive(Clone, Copy)]
pub struct InstallBlockedNoIntegrityFlags {
    pub has_direct: bool,
    pub has_transitive: bool,
    pub has_any: bool,
}

#[derive(Clone, Copy)]
pub struct InstallBlockedMissingFromLockfileFlags {
    pub has_direct: bool,
    pub has_transitive: bool,
    pub has_any: bool,
}

#[derive(Clone, Copy)]
pub struct InstallBlockedUnverifiableGuidanceFlags {
    pub no_integrity: InstallBlockedNoIntegrityFlags,
    pub missing_from_lockfile: InstallBlockedMissingFromLockfileFlags,
    pub has_registry_unavailable: bool,
    pub has_legacy_sha1_lockfile: bool,
    pub has_provenance_inconsistent: bool,
}

#[derive(Clone, Copy)]
pub struct PrintTemplateLineParams<'a> {
    pub template: &'a str,
    pub template_args: &'a [String],
}

#[derive(Clone, Copy)]
pub struct PrintDimmedGuidanceLineParams<'a> {
    pub guidance: &'a str,
}

#[derive(Clone, Copy)]
pub struct PrintParentScopedGuidanceParams<'a> {
    pub has_group: bool,
    pub has_direct: bool,
    pub has_transitive: bool,
    pub direct_guidance: &'a str,
    pub transitive_guidance: &'a str,
    pub fallback_guidance: &'a str,
}

pub enum ParentScopedGuidanceMode<'a> {
    Skip,
    DirectOnly {
        direct_guidance: &'a str,
    },
    TransitiveOnly {
        transitive_guidance: &'a str,
    },
    DirectAndTransitive {
        direct_guidance: &'a str,
        transitive_guidance: &'a str,
    },
    FallbackOnly {
        fallback_guidance: &'a str,
    },
}

#[derive(Clone, Copy)]
pub struct PrintVerificationProgressParams {
    pub completed: usize,
    pub total: usize,
    pub percentage: usize,
}

#[derive(Clone, Copy)]
pub struct ShouldRenderProgressBarParams<'a> {
    pub output_format: &'a OutputFormat,
    pub quiet: bool,
}

#[derive(Clone, Copy)]
pub struct PrintPostVerifyElapsedWarningParams<'a> {
    pub command_name: &'a str,
    pub package_count: usize,
    pub elapsed_secs: u64,
    pub good_term_secs: u64,
}

#[derive(Clone, Copy)]
pub struct PrintInstallCandidateResolvedParams<'a> {
    pub requested_spec: &'a str,
    pub resolved_candidate: &'a PackageRef,
    pub transitive_count: usize,
}
