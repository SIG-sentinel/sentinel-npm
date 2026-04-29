use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::package::PackageRef;
use super::report::Report;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonVerdict {
    Clean,
    Compromised,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub package: PackageRef,
    pub verdict: Verdict,
    pub detail: String,
    pub evidence: Evidence,
    #[serde(default)]
    pub is_direct: bool,
    #[serde(default)]
    pub direct_parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tarball_fingerprint: Option<String>,
}

impl VerifyResult {
    pub fn is_clean(&self) -> bool {
        self.verdict == Verdict::Clean
    }

    pub fn is_compromised(&self) -> bool {
        matches!(self.verdict, Verdict::Compromised { .. })
    }

    pub fn is_unverifiable(&self) -> bool {
        matches!(self.verdict, Verdict::Unverifiable { .. })
    }

    pub fn is_blocking_unverifiable(&self) -> bool {
        self.is_unverifiable() && !self.is_provenance_missing()
    }

    pub fn unverifiable_reason(&self) -> Option<&UnverifiableReason> {
        match &self.verdict {
            Verdict::Unverifiable { reason } => Some(reason),
            _ => None,
        }
    }

    pub fn is_unverifiable_with_reason(&self, reason: UnverifiableReason) -> bool {
        matches!(&self.verdict, Verdict::Unverifiable { reason: r } if *r == reason)
    }

    pub fn is_no_integrity_field(&self) -> bool {
        self.is_unverifiable_with_reason(UnverifiableReason::NoIntegrityField)
    }

    pub fn is_direct_no_integrity_field(&self) -> bool {
        self.is_direct && self.is_no_integrity_field()
    }

    pub fn is_transitive_no_integrity_field(&self) -> bool {
        !self.is_direct && self.is_no_integrity_field()
    }

    pub fn is_missing_from_lockfile(&self) -> bool {
        self.is_unverifiable_with_reason(UnverifiableReason::MissingFromLockfile)
    }

    pub fn is_direct_missing_from_lockfile(&self) -> bool {
        self.is_direct && self.is_missing_from_lockfile()
    }

    pub fn is_transitive_missing_from_lockfile(&self) -> bool {
        !self.is_direct && self.is_missing_from_lockfile()
    }

    pub fn is_registry_unavailable(&self) -> bool {
        matches!(
            self.verdict,
            Verdict::Unverifiable {
                reason: UnverifiableReason::RegistryOffline
                    | UnverifiableReason::RegistryTimeout
                    | UnverifiableReason::TarballTooLarge
            }
        )
    }

    pub fn is_legacy_sha1_lockfile(&self) -> bool {
        self.is_unverifiable_with_reason(UnverifiableReason::LegacySha1Lockfile)
    }

    pub fn is_provenance_missing(&self) -> bool {
        self.is_unverifiable_with_reason(UnverifiableReason::ProvenanceMissing)
    }

    pub fn is_provenance_inconsistent(&self) -> bool {
        self.is_unverifiable_with_reason(UnverifiableReason::ProvenanceInconsistent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Clean,
    Unverifiable {
        reason: UnverifiableReason,
    },
    Compromised {
        expected: String,
        actual: String,
        source: CompromisedSource,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnverifiableReason {
    NoIntegrityField,
    LegacySha1Lockfile,
    RegistryOffline,
    RegistryTimeout,
    MissingFromLockfile,
    TarballTooLarge,
    ProvenanceMissing,
    ProvenanceInconsistent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompromisedSource {
    LockfileVsRegistry,
    DownloadVsRegistry,
    InstalledVsLockfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub registry_integrity: Option<String>,
    pub lockfile_integrity: Option<String>,
    pub computed_sha512: Option<String>,
    pub source_url: Option<String>,
    pub provenance_subject_digest: Option<String>,
    pub provenance_issuer: Option<String>,
    pub provenance_identity: Option<String>,
    pub provenance_bundle_source: Option<String>,
}

impl Evidence {
    pub fn empty() -> Self {
        Self {
            registry_integrity: None,
            lockfile_integrity: None,
            computed_sha512: None,
            source_url: None,
            provenance_subject_digest: None,
            provenance_issuer: None,
            provenance_identity: None,
            provenance_bundle_source: None,
        }
    }
}

pub struct VerifyResultWithTarball {
    pub result: VerifyResult,
    pub tarball: Option<VerifiedTarball>,
}

pub enum VerifiedTarball {
    Memory(Vec<u8>),
    Spool(PathBuf),
}

pub struct CreateUnverifiableParams<'a> {
    pub reason: UnverifiableReason,
    pub package: &'a PackageRef,
    pub detail_template: &'a str,
    pub template_args: &'a [String],
    pub evidence: Evidence,
}

pub trait VerdictFilter {
    fn get_compromised(&self) -> Vec<&VerifyResult>;
    fn get_unverifiable(&self) -> Vec<&VerifyResult>;
}

impl VerdictFilter for Report {
    fn get_compromised(&self) -> Vec<&VerifyResult> {
        self.results
            .iter()
            .filter(|result| result.is_compromised())
            .collect()
    }

    fn get_unverifiable(&self) -> Vec<&VerifyResult> {
        self.results
            .iter()
            .filter(|result| result.is_unverifiable())
            .collect()
    }
}
