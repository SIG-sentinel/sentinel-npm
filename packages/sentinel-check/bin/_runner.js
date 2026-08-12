#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { ensureManagedBinary } = require("./_managedBinary");

const PACKAGE_ROOT = path.resolve(__dirname, "..");
const PACKAGE_MANIFEST = require(path.join(PACKAGE_ROOT, "package.json"));
const EXIT_FAILURE = 1;
const VERSION_PATTERN = /^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;
const PLATFORM_ASSETS = Object.freeze({
  "linux-x64": { assetName: "sentinel-linux-x64", binaryName: "sentinel" },
  "darwin-x64": { assetName: "sentinel-darwin-x64", binaryName: "sentinel" },
  "darwin-arm64": { assetName: "sentinel-darwin-arm64", binaryName: "sentinel" },
  "win32-x64": { assetName: "sentinel-windows-x64.exe", binaryName: "sentinel.exe" },
});
const STRICT_MODE_FALLBACK_MESSAGES = [
  "sentinel: strict mode requires a verified, managed binary.",
  "sentinel: no checksum-verified managed binary is available.",
  "sentinel: install Sentinel via: npm install --save-dev @sentinel/sentinel",
  "sentinel: or set SENTINEL_BIN=/absolute/path/to/verified/binary",
];
const NO_BINARY_FOUND_MESSAGES = [
  "sentinel: could not find the Sentinel binary.",
  "Install Sentinel first or set SENTINEL_BIN=/absolute/path/to/sentinel.",
  "For local development, run: cargo build --release.",
  "For CI/CD, publish GitHub release assets and rerun via npx --package <pkg> sentinel.",
];

function resolveVersion() {
  const requested = process.env.SENTINEL_VERSION || PACKAGE_MANIFEST.version;
  const hasVersionPrefix = requested.startsWith("v");
  const raw = hasVersionPrefix ? requested : `v${requested}`;

  const isValidVersion = VERSION_PATTERN.test(raw);

  if (!isValidVersion) {
    throw new Error(`SENTINEL_VERSION "${raw}" is not a valid version (expected vX.Y.Z)`);
  }

  return raw;
}

function normalizeRepository(repository) {
  if (!repository) return null;

  const raw = typeof repository === "string" ? repository : repository.url;

  if (!raw) return null;

  const trimmed = raw.replace(/^git\+/, "").replace(/\.git$/, "");
  const httpsMatch = trimmed.match(/github\.com[/:]([^/]+\/[^/]+)$/);

  if (httpsMatch) return httpsMatch[1];

  return null;
}

function resolveReleaseRepo() {
  const unsafeOverrideAllowed = process.env.SENTINEL_ALLOW_UNSAFE_RELEASE_OVERRIDE === "1";

  if (unsafeOverrideAllowed && process.env.SENTINEL_RELEASE_REPO) {
    return process.env.SENTINEL_RELEASE_REPO;
  }

  return normalizeRepository(PACKAGE_MANIFEST.repository);
}

function resolveBaseUrl() {
  const override = process.env.SENTINEL_RELEASE_BASE_URL;
  const unsafeOverrideAllowed = process.env.SENTINEL_ALLOW_UNSAFE_RELEASE_OVERRIDE === "1";

  if (override && unsafeOverrideAllowed) return override.replace(/\/$/, "");

  const repo = resolveReleaseRepo();
  const version = resolveVersion();

  if (!repo) return null;

  return `https://github.com/${repo}/releases/download/${version}`;
}

function resolvePlatformAsset() {
  const platformKey = `${process.platform}-${process.arch}`;

  return PLATFORM_ASSETS[platformKey] || null;
}

function resolveCacheDir() {
  const explicit = process.env.SENTINEL_CACHE_DIR;

  if (explicit) return explicit;

  const windowsCacheBase = process.env.LOCALAPPDATA || os.homedir();
  const unixCacheBase = process.env.XDG_CACHE_HOME || path.join(os.homedir(), ".cache");
  const cacheBase = process.platform === "win32" ? windowsCacheBase : unixCacheBase;

  return path.join(cacheBase, "sentinel", "bin");
}

function resolveManagedBinaryPath() {
  const platformAsset = resolvePlatformAsset();

  if (!platformAsset) return null;

  return path.join(resolveCacheDir(), resolveVersion(), platformAsset.assetName);
}

function resolveCandidates() {
  const fromEnv = process.env.SENTINEL_BIN;
  const cwd = process.cwd();

  return [
    fromEnv,
    path.resolve(cwd, "target/release/sentinel"),
    path.resolve(cwd, "target/debug/sentinel")
  ].filter(Boolean);
}

function canUseBinary(candidate) {
  if (!candidate.includes("/")) return true;

  return fs.existsSync(candidate);
}

function shouldSkipDownload() {
  return process.env.SENTINEL_SKIP_DOWNLOAD === "1";
}

function printLines(lines) {
  for (const line of lines) {
    console.error(line);
  }
}

function tryCandidateAndExit(candidate, args) {
  const result = spawnSync(candidate, args, { stdio: "inherit" });
  const isMissingBinary = result.error?.code === "ENOENT";

  if (isMissingBinary) return false;

  if (result.error) {
    console.error(`sentinel: failed to execute '${candidate}': ${result.error.message}`);
    process.exit(EXIT_FAILURE);
  }

  process.exit(result.status ?? EXIT_FAILURE);
}

async function runSentinel(args) {
  try {
    const managedBinary = await ensureManagedBinary({
      shouldSkip: shouldSkipDownload(),
      platformAsset: resolvePlatformAsset(),
      baseUrl: resolveBaseUrl(),
      managedBinaryPath: resolveManagedBinaryPath(),
      version: resolveVersion(),
      platform: process.platform,
    });
    const hasManagedBinary = managedBinary != null;

    if (hasManagedBinary) {
      const result = spawnSync(managedBinary, args, { stdio: "inherit" });
      const hasError = result.error != null;

      if (hasError) {
        const errorMsg = `sentinel: failed to execute downloaded binary: ${result.error.message}`;

        console.error(errorMsg);
        process.exit(EXIT_FAILURE);
      }

      const exitCode = result.status ?? EXIT_FAILURE;
      process.exit(exitCode);
    }
  } catch (error) {
    console.error(`sentinel: ${error.message}`);
    process.exit(EXIT_FAILURE);
  }

  const candidates = resolveCandidates();
  for (const candidate of candidates) {
    if (!canUseBinary(candidate)) continue;

    const success = tryCandidateAndExit(candidate, args);

    if (!success) continue;
  }

  printLines(STRICT_MODE_FALLBACK_MESSAGES);
  process.exit(EXIT_FAILURE);
}

module.exports = {
  runSentinel
};
