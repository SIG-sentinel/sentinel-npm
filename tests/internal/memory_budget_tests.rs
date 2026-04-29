use super::*;

const BUDGET_1_MIB: usize = 1024 * 1024;
const HALF_MIB: usize = 512 * 1024;
const QUARTER_MIB: usize = 256 * 1024;
const BUDGET_TINY: usize = 512;

#[test]
fn test_memory_budget_tracker_creation() {
    let tracker = MemoryBudgetTracker::new(BUDGET_1_MIB);
    assert_eq!(tracker.current_bytes(), 0);
    assert_eq!(tracker.get_budget(), BUDGET_1_MIB);
}

#[test]
fn test_memory_budget_tracking() {
    let tracker = MemoryBudgetTracker::new(BUDGET_1_MIB);
    tracker.record_buffer(HALF_MIB);
    assert_eq!(tracker.current_bytes(), HALF_MIB);
    tracker.release_buffer(QUARTER_MIB);
    assert_eq!(tracker.current_bytes(), QUARTER_MIB);
}

#[test]
fn test_should_fallback_memory_mode() {
    let tracker = MemoryBudgetTracker::new(BUDGET_TINY);
    tracker.record_buffer(BUDGET_TINY);
    let decision = tracker.should_fallback_to_spool(ArtifactStore::Memory);
    assert_eq!(decision.effective_mode, ArtifactStore::Spool);
    assert!(decision.fell_back);
}

#[test]
fn test_should_not_fallback_spool_explicit() {
    let tracker = MemoryBudgetTracker::new(BUDGET_TINY);
    tracker.record_buffer(BUDGET_TINY);
    let decision = tracker.should_fallback_to_spool(ArtifactStore::Spool);
    assert_eq!(decision.effective_mode, ArtifactStore::Spool);
    assert!(!decision.fell_back);
}

#[test]
fn test_auto_mode_fallback() {
    let tracker = MemoryBudgetTracker::new(100);
    tracker.record_buffer(100);
    let decision = tracker.should_fallback_to_spool(ArtifactStore::Auto);
    assert_eq!(decision.effective_mode, ArtifactStore::Spool);
    assert!(decision.fell_back);
}
