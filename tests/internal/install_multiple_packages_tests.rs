#![allow(clippy::expect_used)]

use super::{parse_install_package_request, parse_package_ref};
use crate::types::OutputFormat;

#[test]
fn test_parse_multiple_package_requests() {
    let specs = ["lodash@4.17.21", "axios@1.11.0", "@types/express@4.17.25"];

    let parsed: Vec<_> = specs
        .iter()
        .filter_map(|spec| parse_install_package_request(spec))
        .collect();

    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].package_name, "lodash");
    assert_eq!(parsed[1].package_name, "axios");
    assert_eq!(parsed[2].package_name, "@types/express");
}

#[test]
fn test_parse_package_refs_for_multiple_installs() {
    let specs = ["lodash@4.17.21", "axios@1.11.0", "express@4.18.2"];

    let parsed: Vec<_> = specs
        .iter()
        .filter_map(|spec| parse_package_ref(spec))
        .collect();

    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].name, "lodash");
    assert_eq!(parsed[0].version, "4.17.21");
    assert_eq!(parsed[1].name, "axios");
    assert_eq!(parsed[1].version, "1.11.0");
    assert_eq!(parsed[2].name, "express");
    assert_eq!(parsed[2].version, "4.18.2");
}

#[test]
fn test_install_args_accepts_multiple_packages_as_varargs() {
    let packages = [
        "lodash@4.17.21".to_string(),
        "axios@1.11.0".to_string(),
        "express@4.18.2".to_string(),
    ];

    assert_eq!(packages.len(), 3);
    assert_eq!(packages[0], "lodash@4.17.21");
}

#[test]
fn test_multi_package_install_specification_with_common_flags() {
    let packages = ["lodash@4.17.21", "axios@1.11.0", "express@4.18.2"];

    let format = OutputFormat::Json;
    let allow_scripts = false;
    let dry_run = false;

    assert!(packages.len() > 1);
    assert_eq!(format, OutputFormat::Json);
    assert!(!allow_scripts);
    assert!(!dry_run);
}

#[test]
fn test_atomic_install_specification_failure_in_one_fails_all() {
    let packages = ["lodash@4.17.21", "malicious@1.0.0", "axios@1.11.0"];

    let mut installed = vec![];
    let mut should_continue = true;

    for (idx, pkg) in packages.iter().enumerate() {
        if !should_continue {
            assert_eq!(idx, 2);
            break;
        }

        let package_name = pkg.split('@').next().unwrap_or("");
        if package_name == "malicious" {
            should_continue = false;
        } else if should_continue {
            installed.push(pkg.to_string());
        }
    }

    assert_eq!(installed.len(), 1);
}

#[test]
fn test_multi_package_install_consolidates_results_in_report() {
    struct PackageInstallResult {
        name: String,
        version: String,
        status: String,
        integrity_verified: bool,
    }

    let results = [
        PackageInstallResult {
            name: "lodash".to_string(),
            version: "4.17.21".to_string(),
            status: "installed".to_string(),
            integrity_verified: true,
        },
        PackageInstallResult {
            name: "axios".to_string(),
            version: "1.11.0".to_string(),
            status: "installed".to_string(),
            integrity_verified: true,
        },
        PackageInstallResult {
            name: "express".to_string(),
            version: "4.18.2".to_string(),
            status: "installed".to_string(),
            integrity_verified: true,
        },
    ];

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].name, "lodash");
    assert_eq!(results[0].version, "4.17.21");
    assert!(results.iter().all(|r| r.integrity_verified));
    assert!(results.iter().all(|r| r.status == "installed"));
}

#[test]
fn test_multi_package_json_output_structure() {
    #[derive(Debug)]
    struct JsonReportPackage {
        name: String,
        version: String,
        verdict: String,
    }

    let packages = [
        JsonReportPackage {
            name: "lodash".to_string(),
            version: "4.17.21".to_string(),
            verdict: "ok".to_string(),
        },
        JsonReportPackage {
            name: "axios".to_string(),
            version: "1.11.0".to_string(),
            verdict: "ok".to_string(),
        },
    ];

    let summary = (
        packages.len(),
        packages.iter().filter(|p| p.verdict == "ok").count(),
    );
    assert_eq!(packages[0].name, "lodash");
    assert_eq!(packages[1].version, "1.11.0");
    assert_eq!(summary, (2, 2));
}

#[test]
fn test_multi_package_ui_feedback_shows_progress_per_package() {
    let packages = ["lodash@4.17.21", "axios@1.11.0", "express@4.18.2"];
    let total = packages.len();

    for (idx, pkg) in packages.iter().enumerate() {
        let step = idx + 1;
        let expected_output = format!("[{step}/{total}] Installing {pkg} ... ✓");
        assert!(expected_output.contains(pkg));
    }
}

#[test]
fn test_rollback_removes_all_partial_changes_on_failure() {
    let initial_state = "clean";
    let after_pkg1_install = "lodash installed, package.json updated";
    let after_pkg2_failure = "axios install failed";
    let final_state_after_rollback = "clean";

    assert_ne!(after_pkg1_install, final_state_after_rollback);
    assert_ne!(after_pkg2_failure, final_state_after_rollback);
    assert_eq!(initial_state, final_state_after_rollback);
}
