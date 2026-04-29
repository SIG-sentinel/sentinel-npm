use semver::Version;
use semver::VersionReq;

use crate::constants::PACKAGE_SPEC_SEPARATOR;
use crate::constants::SEMVER_PINNED_EXTRA_CHARS;
use crate::constants::SEMVER_RANGE_CHARS;
use crate::constants::SEMVER_VERSION_PREFIX;
use crate::constants::paths::{PACKAGE_VERSION_LATEST, PACKAGE_VERSION_NEXT};
use crate::types::{
    CandidateMatchScope, CollectInstallPackagesParams, DependencyNode, DependencyTree,
    InstallPackageRequest, PackageRef, VersionSpecKind,
};

pub(super) fn normalize_semver_input(value: &str) -> &str {
    value.trim().trim_start_matches(SEMVER_VERSION_PREFIX)
}

pub(super) fn parse_semver_version(value: &str) -> Option<Version> {
    Version::parse(normalize_semver_input(value)).ok()
}

pub(super) fn is_exact_version_spec(spec: &str) -> bool {
    let has_range_tokens = spec.chars().any(|c| SEMVER_RANGE_CHARS.contains(&c));
    let has_digits = spec.chars().any(|c| c.is_ascii_digit());
    let has_only_valid_chars = spec
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || SEMVER_PINNED_EXTRA_CHARS.contains(&c));

    !has_range_tokens && has_digits && has_only_valid_chars
}

fn is_tag_spec(spec: &str) -> bool {
    let normalized = spec.to_ascii_lowercase();

    normalized == PACKAGE_VERSION_LATEST || normalized == PACKAGE_VERSION_NEXT
}

#[cfg(test)]
pub(super) fn parse_package_ref(spec: &str) -> Option<PackageRef> {
    let separator = spec.rfind(PACKAGE_SPEC_SEPARATOR)?;
    let package_name = &spec[..separator];
    let package_version = &spec[separator + PACKAGE_SPEC_SEPARATOR.len_utf8()..];

    let is_missing_package_name = package_name.is_empty();
    let is_missing_package_version = package_version.is_empty();
    let has_missing_package_parts = is_missing_package_name || is_missing_package_version;

    if has_missing_package_parts {
        return None;
    }

    Some(PackageRef::new(package_name, package_version))
}

pub(super) fn parse_install_package_request(spec: &str) -> Option<InstallPackageRequest> {
    let trimmed = spec.trim();
    let is_empty_input = trimmed.is_empty();
    let has_whitespace = trimmed.chars().any(char::is_whitespace);
    let ends_with_separator = trimmed.ends_with(PACKAGE_SPEC_SEPARATOR);
    let has_invalid_input = is_empty_input || has_whitespace || ends_with_separator;

    if has_invalid_input {
        return None;
    }

    let separator = trimmed.rfind(PACKAGE_SPEC_SEPARATOR);
    let starts_scoped = trimmed.starts_with(PACKAGE_SPEC_SEPARATOR);

    let (package_name, version_spec) = match separator {
        Some(0) if starts_scoped => (trimmed.to_string(), None),
        Some(index) => {
            let package_name = &trimmed[..index];
            let package_version = &trimmed[index + PACKAGE_SPEC_SEPARATOR.len_utf8()..];

            let is_missing_package_name = package_name.is_empty();
            let is_missing_package_version = package_version.is_empty();
            let has_missing_package_parts = is_missing_package_name || is_missing_package_version;

            if has_missing_package_parts {
                return None;
            }

            (package_name.to_string(), Some(package_version.to_string()))
        }
        None => (trimmed.to_string(), None),
    };

    let install_package_request = InstallPackageRequest {
        package_name,
        version_spec,
    };

    Some(install_package_request)
}

pub(super) fn package_key(package_ref: &PackageRef) -> String {
    package_ref.to_string()
}

fn resolve_exact_spec(spec: &str, matches: &mut Vec<PackageRef>) -> Option<PackageRef> {
    let exact_match = matches.iter().find(|p| p.version == *spec).cloned();
    let normalized_match = parse_semver_version(spec).and_then(|requested| {
        matches
            .iter()
            .find(|p| parse_semver_version(&p.version) == Some(requested.clone()))
            .cloned()
    });

    exact_match.or(normalized_match).or_else(|| {
        matches.sort_by(|left, right| left.version.cmp(&right.version));
        matches.pop()
    })
}

fn resolve_range_spec(spec: &str, matches: &[PackageRef]) -> Option<PackageRef> {
    let version_req = VersionReq::parse(spec).ok()?;

    matches
        .iter()
        .filter_map(|package_ref| {
            parse_semver_version(&package_ref.version).map(|version| (version, package_ref.clone()))
        })
        .filter(|(version, _)| version_req.matches(version))
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, package_ref)| package_ref)
}

fn select_highest_semver_candidate(matches: &[PackageRef]) -> Option<PackageRef> {
    matches
        .iter()
        .filter_map(|package_ref| {
            parse_semver_version(&package_ref.version).map(|version| (version, package_ref.clone()))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, package_ref)| package_ref)
}

fn classify_version_spec(version_spec: Option<&String>) -> VersionSpecKind<'_> {
    let Some(spec) = version_spec else {
        return VersionSpecKind::Unspecified;
    };

    let is_tag_version_spec = is_tag_spec(spec);
    let is_exact_version = is_exact_version_spec(spec);

    match (is_tag_version_spec, is_exact_version) {
        (true, _) => VersionSpecKind::Tag,
        (false, true) => VersionSpecKind::Exact(spec),
        (false, false) => VersionSpecKind::Range(spec),
    }
}

fn classify_candidate_match_scope(direct_matches: &[PackageRef]) -> CandidateMatchScope {
    if direct_matches.is_empty() {
        CandidateMatchScope::IncludeTransitive
    } else {
        CandidateMatchScope::DirectOnly
    }
}

pub(super) fn resolve_install_candidate_package(
    dependency_tree: &DependencyTree,
    request: &InstallPackageRequest,
) -> Option<PackageRef> {
    let InstallPackageRequest {
        package_name,
        version_spec,
    } = request;

    let direct_matches: Vec<PackageRef> = dependency_tree
        .nodes
        .values()
        .filter(|node| node.package.name == *package_name && node.is_direct)
        .map(|node| node.package.clone())
        .collect();

    let candidate_match_scope = classify_candidate_match_scope(&direct_matches);

    let mut candidate_matches = match candidate_match_scope {
        CandidateMatchScope::DirectOnly => direct_matches,
        CandidateMatchScope::IncludeTransitive => dependency_tree
            .nodes
            .values()
            .filter(|node| node.package.name == *package_name)
            .map(|node| node.package.clone())
            .collect(),
    };

    (!candidate_matches.is_empty())
        .then(|| match classify_version_spec(version_spec.as_ref()) {
            VersionSpecKind::Unspecified | VersionSpecKind::Tag => {
                select_highest_semver_candidate(&candidate_matches)
            }
            VersionSpecKind::Exact(spec) => resolve_exact_spec(spec, &mut candidate_matches),
            VersionSpecKind::Range(spec) => {
                resolve_range_spec(spec, &candidate_matches).or_else(|| {
                    candidate_matches.sort_by(|left, right| left.version.cmp(&right.version));
                    candidate_matches.pop()
                })
            }
        })
        .flatten()
}

pub(super) fn collect_install_packages_to_verify(
    params: CollectInstallPackagesParams<'_>,
) -> Option<Vec<DependencyNode>> {
    let CollectInstallPackagesParams {
        dependency_tree,
        package_reference,
    } = params;

    let target_key = package_reference.to_string();
    let target_node = dependency_tree.nodes.get(&target_key)?;
    let mut keys_to_verify = dependency_tree.get_transitive_deps(&target_node.package);

    keys_to_verify.insert(target_key);

    let mut packages_to_verify: Vec<_> = keys_to_verify
        .iter()
        .filter_map(|key| dependency_tree.nodes.get(key).cloned())
        .collect();

    packages_to_verify
        .sort_by(|left, right| left.package.to_string().cmp(&right.package.to_string()));

    Some(packages_to_verify)
}
