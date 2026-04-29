use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::PackageManager;

#[derive(Debug, Clone, clap::ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HistoryOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryRunMetadata {
    pub run_started_at: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryLockfileMetadata {
    pub path: String,
    pub sha256_before: Option<String>,
    pub sha256_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryPackageMetadata {
    pub name: String,
    pub version: String,
    pub direct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEvent {
    pub schema_version: u32,
    pub event_id: String,
    pub run: HistoryRunMetadata,
    pub occurred_at: String,
    pub project_root: String,
    pub package_manager: String,
    pub command: String,
    pub sentinel_version: String,
    pub lockfile: HistoryLockfileMetadata,
    pub package: HistoryPackageMetadata,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryQuery {
    pub from: String,
    pub to: String,
    pub package: Option<String>,
    pub version: Option<String>,
    pub project: Option<String>,
    pub package_manager: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryPackageModeOutput {
    pub query: HistoryQuery,
    pub found: bool,
    pub matches: Vec<HistoryEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRangeSummary {
    pub events: usize,
    pub projects: usize,
    pub unique_packages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRangeModeOutput {
    pub query: HistoryQuery,
    pub summary: HistoryRangeSummary,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryRunMode {
    Package,
    Range,
}

#[derive(Clone, Copy)]
pub struct RenderPackageModeParams<'a> {
    pub output: &'a HistoryPackageModeOutput,
    pub format: &'a HistoryOutputFormat,
    pub quiet: bool,
}

#[derive(Clone, Copy)]
pub struct RenderRangeModeParams<'a> {
    pub output: &'a HistoryRangeModeOutput,
    pub format: &'a HistoryOutputFormat,
    pub quiet: bool,
}

#[derive(Clone, Copy)]
pub struct ShouldPrintHistoryOutputParams<'a> {
    pub format: &'a HistoryOutputFormat,
    pub quiet: bool,
}

#[derive(Clone, Copy)]
pub struct PrintPackageModeJsonOutputParams<'a> {
    pub output: &'a HistoryPackageModeOutput,
}

#[derive(Clone, Copy)]
pub struct PrintRangeModeJsonOutputParams<'a> {
    pub output: &'a HistoryRangeModeOutput,
}

pub struct HistoryQueryFilters {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub package: Option<String>,
    pub version: Option<String>,
    pub project: Option<String>,
    pub package_manager: Option<String>,
}

pub struct EventMatchesFiltersParams<'a> {
    pub event: &'a HistoryEvent,
    pub occurred_at: DateTime<Utc>,
    pub filters: &'a HistoryQueryFilters,
}

#[derive(Clone, Copy)]
pub struct AppendHistoryEventsParams<'a> {
    pub current_working_directory: &'a Path,
    pub package_manager: PackageManager,
    pub command: &'a str,
    pub lockfile_path: &'a str,
    pub lock_hash_before: &'a Option<String>,
    pub lock_hash_after: &'a Option<String>,
    pub packages: &'a [HistoryPackageMetadata],
}

#[derive(Clone, Copy)]
pub struct RetainLastNParams<'a> {
    pub events: &'a [HistoryEvent],
    pub max_per_key: usize,
}

#[derive(Clone, Copy)]
pub struct AppendEventsImplParams<'a> {
    pub ledger_path: &'a Path,
    pub events: &'a [HistoryEvent],
}
