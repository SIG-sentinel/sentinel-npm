use crate::types::ComparisonVerdict;

pub fn compare_integrity(expected: &str, actual: &str) -> ComparisonVerdict {
    if expected == actual {
        ComparisonVerdict::Clean
    } else {
        ComparisonVerdict::Compromised
    }
}
