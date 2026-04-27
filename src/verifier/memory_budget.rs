use crate::types::{ArtifactStore, FallbackDecision};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use crate::types::MemoryBudgetTracker;

const DEFAULT_MEMORY_BUDGET_BYTES: usize = 512 * 1024 * 1024;
const CGROUP_MEMORY_MAX_PATH: &str = "/sys/fs/cgroup/memory.max";
const PROC_MEMINFO_PATH: &str = "/proc/meminfo";
const NO_CGROUP_LIMIT_VALUE: &str = "max";
const NO_CGROUP_MEMORY_LIMIT_ERROR: &str = "No cgroup memory limit";
const MEM_AVAILABLE_PREFIX: &str = "MemAvailable:";
const MEM_AVAILABLE_NOT_FOUND_ERROR: &str = "MemAvailable not found in /proc/meminfo";
const MEMINFO_VALUE_INDEX: usize = 1;
const KIBIBYTE_IN_BYTES: usize = 1024;
const CONTAINER_MEMORY_BUDGET_NUMERATOR: usize = 3;
const CONTAINER_MEMORY_BUDGET_DENOMINATOR: usize = 4;
const SYSTEM_MEMORY_BUDGET_NUMERATOR: usize = 1;
const SYSTEM_MEMORY_BUDGET_DENOMINATOR: usize = 2;

impl MemoryBudgetTracker {
    pub fn new(max_budget_bytes: usize) -> Self {
        Self {
            inflight_bytes: Arc::new(AtomicUsize::new(0)),
            max_budget_bytes,
        }
    }

    pub fn should_fallback_to_spool(&self, proposed_mode: ArtifactStore) -> FallbackDecision {
        match proposed_mode {
            ArtifactStore::Spool => FallbackDecision {
                effective_mode: ArtifactStore::Spool,
                fell_back: false,
            },
            ArtifactStore::Memory | ArtifactStore::Auto => {
                let current_bytes = self.inflight_bytes.load(Ordering::SeqCst);
                let budget_exceeded = current_bytes >= self.max_budget_bytes;
                let mut fallback_decision = FallbackDecision {
                    effective_mode: ArtifactStore::Memory,
                    fell_back: false,
                };

                if budget_exceeded {
                    fallback_decision = FallbackDecision {
                        effective_mode: ArtifactStore::Spool,
                        fell_back: true,
                    };
                }

                fallback_decision
            }
        }
    }

    pub fn record_buffer(&self, bytes: usize) {
        self.inflight_bytes.fetch_add(bytes, Ordering::SeqCst);
    }

    pub fn release_buffer(&self, bytes: usize) {
        self.inflight_bytes.fetch_sub(bytes, Ordering::SeqCst);
    }

    pub fn current_bytes(&self) -> usize {
        self.inflight_bytes.load(Ordering::SeqCst)
    }

    pub fn get_budget(&self) -> usize {
        self.max_budget_bytes
    }

    pub fn clone_counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.inflight_bytes)
    }
}

pub fn detect_memory_budget() -> usize {
    read_cgroup_memory_limit()
        .ok()
        .map(compute_container_memory_budget)
        .or_else(|| {
            read_meminfo_available()
                .ok()
                .map(compute_system_memory_budget)
        })
        .unwrap_or(DEFAULT_MEMORY_BUDGET_BYTES)
}

fn compute_container_memory_budget(limit_bytes: usize) -> usize {
    compute_ratio_budget(
        limit_bytes,
        CONTAINER_MEMORY_BUDGET_NUMERATOR,
        CONTAINER_MEMORY_BUDGET_DENOMINATOR,
    )
}

fn compute_system_memory_budget(available_bytes: usize) -> usize {
    compute_ratio_budget(
        available_bytes,
        SYSTEM_MEMORY_BUDGET_NUMERATOR,
        SYSTEM_MEMORY_BUDGET_DENOMINATOR,
    )
}

fn compute_ratio_budget(bytes: usize, numerator: usize, denominator: usize) -> usize {
    bytes.saturating_mul(numerator) / denominator
}

fn read_cgroup_memory_limit() -> Result<usize, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(CGROUP_MEMORY_MAX_PATH)?;
    let limit_str = content.trim();

    if limit_str == NO_CGROUP_LIMIT_VALUE {
        return Err(NO_CGROUP_MEMORY_LIMIT_ERROR.into());
    }

    Ok(limit_str.parse()?)
}

fn read_meminfo_available() -> Result<usize, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(PROC_MEMINFO_PATH)?;

    let mem_available_kib = content
        .lines()
        .filter(|line| line.starts_with(MEM_AVAILABLE_PREFIX))
        .filter_map(|line| line.split_whitespace().nth(MEMINFO_VALUE_INDEX))
        .find_map(|value| value.parse::<usize>().ok());

    match mem_available_kib {
        Some(kib) => Ok(kib * KIBIBYTE_IN_BYTES),
        None => Err(MEM_AVAILABLE_NOT_FOUND_ERROR.into()),
    }
}

#[cfg(test)]
#[path = "../../tests/internal/memory_budget_tests.rs"]
mod tests;
