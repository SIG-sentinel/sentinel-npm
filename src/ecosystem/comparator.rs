#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonVerdict {
    Clean,
    Compromised,
}

pub fn compare_integrity(expected: &str, actual: &str) -> ComparisonVerdict {
    if expected == actual {
        ComparisonVerdict::Clean
    } else {
        ComparisonVerdict::Compromised
    }
}
