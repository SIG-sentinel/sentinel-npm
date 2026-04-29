#!/usr/bin/env node

const crypto = require("node:crypto");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const http = require("node:http");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");

const PACKAGE_ROOT = path.resolve(__dirname, "..");
const PACKAGE_MANIFEST = require(path.join(PACKAGE_ROOT, "package.json"));
const DOWNLOAD_TIMEOUT_MS = 30_000;
const EXIT_FAILURE = 1;
const LOCAL_HTTP_HOSTS = new Set(["127.0.0.1", "localhost"]);
const CHECKSUM_FILENAME = "checksums.txt";
const VERSION_PATTERN = /^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;
const CHECKSUM_PATTERN = /^([a-f0-9]{64})\s+\*?(.+)$/i;
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

function resolveDownloadClient(parsedUrl) {
  const isHttps = parsedUrl.protocol === "https:";

  if (isHttps) return https;

  const isHttp = parsedUrl.protocol === "http:";
  const isLocalHost = LOCAL_HTTP_HOSTS.has(parsedUrl.hostname);
  const isAllowedLocalHttp = isHttp && isLocalHost;

  if (isAllowedLocalHttp) return http;

  return null;
}

function trySpawnAndExit(command, commandArgs) {
  const result = spawnSync(command, commandArgs, { stdio: "inherit" });

  if (result.error) return false;

  process.exit(result.status ?? EXIT_FAILURE);
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

const MAX_REDIRECTS = 3;

function fetchBuffer(url, redirectCount = 0) {
  const promise = new Promise((resolve, reject) => {
    const parsedUrl = new URL(url);
    const client = resolveDownloadClient(parsedUrl);

    if (!client) {
      reject(new Error(`unsupported download protocol for ${url}`));

      return;
    }

    const request = client.get(parsedUrl, { timeout: DOWNLOAD_TIMEOUT_MS }, (response) => {
      const statusCode = response.statusCode ?? 0;
      const isRedirectStatus = statusCode >= 300 && statusCode < 400;
      const hasRedirectLocation = response.headers.location != null;
      const shouldFollowRedirect = isRedirectStatus && hasRedirectLocation;

      if (shouldFollowRedirect) {
        response.resume();

        const reachedRedirectLimit = redirectCount >= MAX_REDIRECTS;

        if (reachedRedirectLimit) {
          reject(new Error(`too many redirects (>${MAX_REDIRECTS}) for ${url}`));

          return;
        }

        const nextLocation = response.headers.location;

        resolve(fetchBuffer(nextLocation, redirectCount + 1));

        return;
      }

      const isSuccessStatus = statusCode === 200;

      if (!isSuccessStatus) {
        response.resume();
        reject(new Error(`download failed with status ${statusCode} for ${url}`));

        return;
      }

      const chunks = [];

      response.on("data", (chunk) => chunks.push(chunk));
      response.on("end", () => resolve(Buffer.concat(chunks)));
    });

    request.on("timeout", () => {
      request.destroy(new Error(`request timed out after ${DOWNLOAD_TIMEOUT_MS}ms`));
    });

    request.on("error", reject);
  });

  return promise;
}

function sha256(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

function parseChecksums(text) {
  const entries = new Map();

  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    const isEmptyLine = !trimmed;

    if (isEmptyLine) continue;

    const match = trimmed.match(CHECKSUM_PATTERN);
    const isValidChecksumLine = match != null;

    if (!isValidChecksumLine) continue;

    const filename = match[2].trim();
    const hash = match[1].toLowerCase();

    entries.set(filename, hash);
  }

  return entries;
}

async function ensureManagedBinary() {
  const shouldSkip = shouldSkipDownload();

  if (shouldSkip) return null;

  const platformAsset = resolvePlatformAsset();
  const baseUrl = resolveBaseUrl();
  const managedBinary = resolveManagedBinaryPath();
  const hasPlatform = platformAsset != null;
  const hasUrl = baseUrl != null;
  const hasPath = managedBinary != null;
  const hasAllRequirements = hasPlatform && hasUrl && hasPath;

  if (!hasAllRequirements) return null;

  const alreadyExists = fs.existsSync(managedBinary);

  if (alreadyExists) return managedBinary;

  const targetDir = path.dirname(managedBinary);

  fs.mkdirSync(targetDir, { recursive: true });

  const checksumsUrl = `${baseUrl}/${CHECKSUM_FILENAME}`;
  const assetUrl = `${baseUrl}/${platformAsset.assetName}`;
  const version = resolveVersion();

  process.stderr.write(`sentinel: downloading ${platformAsset.assetName} (${version})\n`);

  const checksumData = await fetchBuffer(checksumsUrl);
  const checksumMap = parseChecksums(checksumData.toString("utf8"));
  const expectedChecksum = checksumMap.get(platformAsset.assetName);
  const checksumFound = expectedChecksum != null;

  if (!checksumFound) {
    throw new Error(`checksum for ${platformAsset.assetName} not found in ${CHECKSUM_FILENAME}`);
  }

  const binaryData = await fetchBuffer(assetUrl);
  const actualChecksum = sha256(binaryData);
  const checksumMatches = actualChecksum === expectedChecksum;

  if (!checksumMatches) {
    throw new Error(`checksum mismatch for ${platformAsset.assetName}`);
  }

  const tempPath = `${managedBinary}.tmp`;

  fs.writeFileSync(tempPath, binaryData);

  const isNotWindows = process.platform !== "win32";

  if (isNotWindows) fs.chmodSync(tempPath, 0o700);

  fs.renameSync(tempPath, managedBinary);

  return managedBinary;
}

async function runSentinel(args) {
  try {
    const managedBinary = await ensureManagedBinary();
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
