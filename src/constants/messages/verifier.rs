pub const VERIFIER_DETAIL_NO_LOCKFILE_INTEGRITY: &str = "{} has no integrity field in lockfile. \
     This may mean the lockfile was generated without integrity \
     hashing (npm < 5) or was manually edited. \
     Prefer regenerating with 'sentinel ci --init-lockfile'.";
pub const VERIFIER_DETAIL_REGISTRY_UNREACHABLE: &str = "Cannot verify {}: npm registry unreachable ({}). \
     The lockfile shows integrity: {} — but we cannot confirm \
     this matches what was originally published.";
pub const VERIFIER_DETAIL_REGISTRY_TIMEOUT: &str = "Cannot verify {}: npm registry timed out ({}ms). \
     The lockfile shows integrity: {} — but we cannot confirm \
     this matches what was originally published.";
pub const VERIFIER_DETAIL_NOT_IN_REGISTRY: &str = "{} not found in npm registry. \
     If this is a private or local package, \
     it cannot be verified against the public registry.";
pub const VERIFIER_DETAIL_REGISTRY_FETCH_ERROR: &str = "Error fetching {} from registry: {}";
pub const VERIFIER_DETAIL_PREDATES_INTEGRITY: &str = "{} predates npm integrity hashing (published before ~2017). \
     Cannot verify cryptographically. \
     Consider replacing with a maintained fork if available.";
pub const VERIFIER_DETAIL_LEGACY_SHA1_LOCKFILE: &str = "{} uses legacy lockfile integrity ({}) and sentinel does not validate sha1 entries.\n\
     Regenerate lockfile integrity to sha512:\n\
     1. Prefer: sentinel ci --init-lockfile\n\
     2. Manual fallback: delete the lockfile and reinstall dependencies\n\
     3. Re-run: sentinel ci";
pub const VERIFIER_DETAIL_CLEAN_LOCKFILE: &str = "{}: three-source integrity verified. \
     lockfile, registry, and downloaded tarball all agree. \
     sha512 confirmed: {}. {} bytes verified.";
pub const VERIFIER_DETAIL_COMPROMISED_LOCKFILE: &str = "CRITICAL: {} integrity mismatch between lockfile and npm registry.\n\
     Lockfile says:    {}\n\
     Registry says:    {}\n\
     This means either the tarball was replaced after publication, \
     or the lockfile was manually modified. \
     Do NOT install. Escalate immediately with package name and integrity evidence: {}";
pub const VERIFIER_DETAIL_COMPROMISED_TARBALL_VS_LOCKFILE: &str = "CRITICAL: Downloaded tarball for {} does NOT match lockfile integrity.\n\
     Lockfile says:        {}\n\
     Downloaded tarball:   {}\n\
     Registry confirms:    {}\n\
     The tarball being served differs from what the lockfile recorded at install time. \
     Possible CDN compromise, registry tampering, or MITM.";
pub const VERIFIER_DETAIL_TARBALL_DOWNLOAD_FAILED_DURING_CHECK: &str = "Cannot fully verify {}: tarball download failed ({}). \
     Lockfile matches registry, but tarball content could not be confirmed.";
pub const VERIFIER_DETAIL_TARBALL_STREAM_ERROR_DURING_CHECK: &str = "Cannot fully verify {}: stream error during tarball download ({}). \
     Lockfile matches registry, but tarball content could not be confirmed.";
pub const VERIFIER_DETAIL_TARBALL_TOO_LARGE_DURING_CHECK: &str = "{} tarball is {}MB, exceeding 50MB safety limit. \
     Lockfile matches registry, but tarball content could not be confirmed due to size.";
pub const VERIFIER_DETAIL_TARBALL_INTEGRITY_FORMAT_ERROR_DURING_CHECK: &str = "{}: integrity format validation failed ({}). \
     Lockfile matches registry, but cryptographic comparison could not be completed.";
pub const VERIFIER_DETAIL_PROVENANCE_MISSING: &str = "{} has no provenance metadata available. \
     Integrity verification passed, but publisher trust evidence is unavailable.";
pub const VERIFIER_DETAIL_PROVENANCE_INCONSISTENT: &str = "{} provenance does not match downloaded artifact.\n\
     Provenance subject: {}\n\
     Downloaded sha512: {}\n\
     This package is blocked because provenance exists but is inconsistent.";
pub const VERIFIER_DETAIL_REGISTRY_FETCH_FAILED: &str = "Cannot fetch {} from registry: {}";
pub const VERIFIER_DETAIL_NO_DIST_INTEGRITY: &str = "{} has no dist.integrity in npm registry. \
     Package predates integrity hashing (~2017). \
     Proceeding without cryptographic verification.";
pub const VERIFIER_DETAIL_TARBALL_DOWNLOAD_FAILED: &str = "Failed to download tarball for {}: {}";
pub const VERIFIER_DETAIL_TARBALL_TOO_LARGE: &str = "{} tarball is {}MB, exceeding 50MB safety limit. \
     This is unusual for an npm package. Verify manually.";
pub const VERIFIER_DETAIL_STREAM_ERROR: &str = "Stream error downloading {}: {}";
pub const VERIFIER_DETAIL_CLEAN_INSTALL: &str =
    "{}: downloaded tarball sha512 matches dist.integrity. {} bytes verified.";
pub const VERIFIER_DETAIL_COMPROMISED_DOWNLOAD: &str = "CRITICAL: Downloaded tarball for {} does NOT match npm registry dist.integrity.\n\
     Expected (registry): {}\n\
     Got (downloaded):    {}\n\
     The tarball being served differs from what was published. \
     Possible MITM, CDN compromise, or registry tampering. \
     Buffer DISCARDED — nothing written to disk.";
pub const VERIFIER_DETAIL_INVALID_INTEGRITY_FORMAT: &str = "dist.integrity field for {} has invalid format: {}. \
     Cannot verify. Treating as UNVERIFIABLE (not CLEAN).";
pub const VERIFIER_TARBALL_OPERATION_ERROR_TEMPLATE: &str = "{} for {}: {}";

pub const RESTORE_ERROR_CONTEXT: &str = "{}: {}";
pub const RESTORE_FAILED_PACKAGE_JSON: &str = "failed to restore package.json";
pub const RESTORE_FAILED_LOCKFILE: &str = "failed to restore {}";
