pub const VERIFIER_DETAIL_NO_LOCKFILE_INTEGRITY: &str = "{} has no integrity field in lockfile. \
     This may mean the lockfile was generated without integrity \
     hashing (npm < 5) or was manually edited.";
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
pub const VERIFIER_DETAIL_CLEAN_LOCKFILE: &str = "{}: lockfile integrity matches npm registry dist.integrity. \
     sha512 confirmed: {}";
pub const VERIFIER_DETAIL_COMPROMISED_LOCKFILE: &str = "CRITICAL: {} integrity mismatch between lockfile and npm registry.\n\
     Lockfile says:    {}\n\
     Registry says:    {}\n\
     This means either the tarball was replaced after publication, \
     or the lockfile was manually modified. \
     Do NOT install. Run: sentinel report {}";
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
