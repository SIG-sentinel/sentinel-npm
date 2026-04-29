use super::{PROVENANCE_MISSING_TOP_N, split_unverifiable_for_text_output};
use crate::types::{Evidence, PackageRef, UnverifiableReason, Verdict, VerifyResult};

fn missing_provenance_result(index: usize) -> VerifyResult {
    VerifyResult {
        package: PackageRef::new(format!("pkg-{index}"), "1.0.0".to_string()),
        verdict: Verdict::Unverifiable {
            reason: UnverifiableReason::ProvenanceMissing,
        },
        detail: "missing provenance".to_string(),
        evidence: Evidence::empty(),
        is_direct: true,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

fn inconsistent_provenance_result() -> VerifyResult {
    VerifyResult {
        package: PackageRef::new("inconsistent", "1.0.0"),
        verdict: Verdict::Unverifiable {
            reason: UnverifiableReason::ProvenanceInconsistent,
        },
        detail: "inconsistent provenance".to_string(),
        evidence: Evidence::empty(),
        is_direct: true,
        direct_parent: None,
        tarball_fingerprint: None,
    }
}

#[test]
fn split_unverifiable_for_text_output_caps_provenance_missing_entries() {
    let mut owned_results: Vec<VerifyResult> =
        (0..12).map(missing_provenance_result).collect::<Vec<_>>();
    owned_results.push(inconsistent_provenance_result());
    let refs: Vec<&VerifyResult> = owned_results.iter().collect();

    let (display, suppressed) = split_unverifiable_for_text_output(&refs);

    assert_eq!(display.len(), PROVENANCE_MISSING_TOP_N + 1);
    assert_eq!(suppressed, 2);
    assert_eq!(
        display
            .iter()
            .filter(|result| result.is_provenance_inconsistent())
            .count(),
        1
    );
}

#[test]
fn split_unverifiable_for_text_output_keeps_all_when_under_limit() {
    let owned_results: Vec<VerifyResult> = (0..3).map(missing_provenance_result).collect();
    let refs: Vec<&VerifyResult> = owned_results.iter().collect();

    let (display, suppressed) = split_unverifiable_for_text_output(&refs);

    assert_eq!(display.len(), 3);
    assert_eq!(suppressed, 0);
}
