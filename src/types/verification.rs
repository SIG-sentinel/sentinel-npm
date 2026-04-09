use serde::{Deserialize, Serialize};

use super::package::PackageRef;
use super::report::Report;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub package: PackageRef,
    pub verdict: Verdict,
    pub detail: String,
    pub evidence: Evidence,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnverifiableReason {
    NoIntegrityField,
    RegistryOffline,
    RegistryTimeout,
    MissingFromLockfile,
    TarballTooLarge,
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
}

impl Evidence {
    pub fn empty() -> Self {
        Self {
            registry_integrity: None,
            lockfile_integrity: None,
            computed_sha512: None,
            source_url: None,
        }
    }
}

pub struct VerifyResultWithTarball {
    pub result: VerifyResult,
    pub tarball: Option<Vec<u8>>,
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
            .filter(|r| matches!(r.verdict, Verdict::Compromised { .. }))
            .collect()
    }

    fn get_unverifiable(&self) -> Vec<&VerifyResult> {
        self.results
            .iter()
            .filter(|r| matches!(r.verdict, Verdict::Unverifiable { .. }))
            .collect()
    }
}
