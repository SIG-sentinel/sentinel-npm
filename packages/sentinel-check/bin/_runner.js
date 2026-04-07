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
const PLATFORM_ASSETS = Object.freeze({
  "linux-x64": { assetName: "sentinel-linux-x64", binaryName: "sentinel" },
  "darwin-x64": { assetName: "sentinel-darwin-x64", binaryName: "sentinel" },
  "darwin-arm64": { assetName: "sentinel-darwin-arm64", binaryName: "sentinel" },
  "win32-x64": { assetName: "sentinel-windows-x64.exe", binaryName: "sentinel.exe" },
});
const STRICT_MODE_FALLBACK_MESSAGES = [
  "sentinel: strict mode blocks unverified fallback binaries.",
  "sentinel: no checksum-verified managed binary is available.",
  "sentinel: to bypass for local debugging only, set SENTINEL_ALLOW_UNVERIFIED_FALLBACK=1.",
];
const NO_BINARY_FOUND_MESSAGES = [
  "sentinel: could not find the Sentinel binary.",
  "Install Sentinel first or set SENTINEL_BIN=/absolute/path/to/sentinel.",
  "For local development, run: cargo build --release.",
  "For CI/CD, publish GitHub release assets and rerun via npx --package <pkg> sentinel.",
];

function resolveVersion() {
  const requested = process.env.SENTINEL_VERSION || PACKAGE_MANIFEST.version;
  const raw = requested.startsWith("v") ? requested : `v${requested}`;

  if (!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(raw)) {
    throw new Error(`SENTINEL_VERSION "${raw}" is not a valid version (expected vX.Y.Z)`);
  }

  return raw;
}

function normalizeRepository(repository) {
  if (!repository) {
    return null;
  }

  const raw = typeof repository === "string" ? repository : repository.url;
  if (!raw) {
    return null;
  }

  const trimmed = raw.replace(/^git\+/, "").replace(/\.git$/, "");
  const httpsMatch = trimmed.match(/github\.com[/:]([^/]+\/[^/]+)$/);
  if (httpsMatch) {
    return httpsMatch[1];
  }

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
  if (override && unsafeOverrideAllowed) {
    return override.replace(/\/$/, "");
  }

  const repo = resolveReleaseRepo();
  const version = resolveVersion();
  if (!repo) {
    return null;
  }

  return `https://github.com/${repo}/releases/download/${version}`;
}

function resolvePlatformAsset() {
  const platformKey = `${process.platform}-${process.arch}`;
  return PLATFORM_ASSETS[platformKey] || null;
}

function resolveCacheDir() {
  const explicit = process.env.SENTINEL_CACHE_DIR;
  if (explicit) {
    return explicit;
  }

  const windowsCacheBase = process.env.LOCALAPPDATA || os.homedir();
  const unixCacheBase = process.env.XDG_CACHE_HOME || path.join(os.homedir(), ".cache");
  const cacheBase = process.platform === "win32" ? windowsCacheBase : unixCacheBase;
  return path.join(cacheBase, "sentinel", "bin");
}

function resolveManagedBinaryPath() {
  const platformAsset = resolvePlatformAsset();
  if (!platformAsset) {
    return null;
  }

  return path.join(resolveCacheDir(), resolveVersion(), platformAsset.assetName);
}

function resolveCandidates() {
  const fromEnv = process.env.SENTINEL_BIN;
  const cwd = process.cwd();

  return [
    fromEnv,
    path.resolve(cwd, "target/release/sentinel"),
    path.resolve(cwd, "target/debug/sentinel"),
    path.resolve(__dirname, "../../../target/release/sentinel"),
    path.resolve(__dirname, "../../../target/debug/sentinel")
  ].filter(Boolean);
}

function canUseBinary(candidate) {
  if (!candidate.includes("/")) {
    return true;
  }
  return fs.existsSync(candidate);
}

function shouldSkipDownload() {
  return process.env.SENTINEL_SKIP_DOWNLOAD === "1";
}

function allowUnverifiedFallback() {
  return process.env.SENTINEL_ALLOW_UNVERIFIED_FALLBACK === "1";
}

function printLines(lines) {
  for (const line of lines) {
    console.error(line);
  }
}

function resolveDownloadClient(parsedUrl) {
  if (parsedUrl.protocol === "https:") {
    return https;
  }

  const isAllowedLocalHttp = parsedUrl.protocol === "http:" && LOCAL_HTTP_HOSTS.has(parsedUrl.hostname);
  if (isAllowedLocalHttp) {
    return http;
  }

  return null;
}

function trySpawnAndExit(command, commandArgs) {
  const result = spawnSync(command, commandArgs, { stdio: "inherit" });
  if (result.error) {
    return false;
  }

  process.exit(result.status ?? EXIT_FAILURE);
}

function tryCandidateAndExit(candidate, args) {
  const result = spawnSync(candidate, args, { stdio: "inherit" });

  const isMissingBinary = result.error?.code === "ENOENT";
  if (isMissingBinary) {
    return false;
  }

  if (result.error) {
    console.error(`sentinel: failed to execute '${candidate}': ${result.error.message}`);
    process.exit(EXIT_FAILURE);
  }

  process.exit(result.status ?? EXIT_FAILURE);
}

const MAX_REDIRECTS = 3;

function fetchBuffer(url, redirectCount = 0) {
  return new Promise((resolve, reject) => {
    const parsedUrl = new URL(url);
    const client = resolveDownloadClient(parsedUrl);

    if (!client) {
      reject(new Error(`unsupported download protocol for ${url}`));
      return;
    }

    const request = client.get(parsedUrl, { timeout: DOWNLOAD_TIMEOUT_MS }, (response) => {
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        response.resume();
        if (redirectCount >= MAX_REDIRECTS) {
          reject(new Error(`too many redirects (>${MAX_REDIRECTS}) for ${url}`));
          return;
        }
        resolve(fetchBuffer(response.headers.location, redirectCount + 1));
        return;
      }

      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`download failed with status ${response.statusCode} for ${url}`));
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
}

function sha256(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

function parseChecksums(text) {
  const entries = new Map();

  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }

    const match = trimmed.match(/^([a-f0-9]{64})\s+\*?(.+)$/i);
    if (!match) {
      continue;
    }

    entries.set(match[2].trim(), match[1].toLowerCase());
  }

  return entries;
}

async function ensureManagedBinary() {
  if (shouldSkipDownload()) {
    return null;
  }

  const platformAsset = resolvePlatformAsset();
  const baseUrl = resolveBaseUrl();
  const managedBinary = resolveManagedBinaryPath();

  if (!platformAsset || !baseUrl || !managedBinary) {
    return null;
  }

  if (fs.existsSync(managedBinary)) {
    return managedBinary;
  }

  const targetDir = path.dirname(managedBinary);
  fs.mkdirSync(targetDir, { recursive: true });

  const checksumsUrl = `${baseUrl}/checksums.txt`;
  const assetUrl = `${baseUrl}/${platformAsset.assetName}`;

  process.stderr.write(`sentinel: downloading ${platformAsset.assetName} (${resolveVersion()})\n`);

  const checksumData = await fetchBuffer(checksumsUrl);
  const checksumMap = parseChecksums(checksumData.toString("utf8"));
  const expectedChecksum = checksumMap.get(platformAsset.assetName);

  if (!expectedChecksum) {
    throw new Error(`checksum for ${platformAsset.assetName} not found in checksums.txt`);
  }

  const binaryData = await fetchBuffer(assetUrl);
  const actualChecksum = sha256(binaryData);
  if (actualChecksum !== expectedChecksum) {
    throw new Error(`checksum mismatch for ${platformAsset.assetName}`);
  }

  const tempPath = `${managedBinary}.tmp`;
  fs.writeFileSync(tempPath, binaryData);
  if (process.platform !== "win32") {
    fs.chmodSync(tempPath, 0o700);
  }
  fs.renameSync(tempPath, managedBinary);
  return managedBinary;
}

async function runSentinel(args) {
  try {
    const managedBinary = await ensureManagedBinary();
    if (managedBinary) {
      const result = spawnSync(managedBinary, args, { stdio: "inherit" });
      if (result.error) {
        console.error(`sentinel: failed to execute downloaded binary: ${result.error.message}`);
        process.exit(EXIT_FAILURE);
      }

      process.exit(result.status ?? EXIT_FAILURE);
    }
  } catch (error) {
    console.error(`sentinel: ${error.message}`);
    process.exit(EXIT_FAILURE);
  }

  if (!allowUnverifiedFallback()) {
    printLines(STRICT_MODE_FALLBACK_MESSAGES);
    process.exit(EXIT_FAILURE);
  }

  const candidates = resolveCandidates();

  for (const candidate of candidates) {
    if (!canUseBinary(candidate)) {
      continue;
    }

    if (!tryCandidateAndExit(candidate, args)) {
      continue;
    }
  }

  if (trySpawnAndExit("sentinel", args)) {
    return;
  }

  const cargoFile = path.resolve(process.cwd(), "Cargo.toml");
  if (fs.existsSync(cargoFile)) {
    if (trySpawnAndExit("cargo", ["run", "--", ...args])) {
      return;
    }
  }

  printLines(NO_BINARY_FOUND_MESSAGES);
  process.exit(EXIT_FAILURE);
}

module.exports = {
  runSentinel
};
