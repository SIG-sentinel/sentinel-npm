#!/usr/bin/env node

const crypto = require("node:crypto");
const fs = require("node:fs");
const http = require("node:http");
const https = require("node:https");
const path = require("node:path");

const DOWNLOAD_TIMEOUT_MS = 30_000;
const CHECKSUM_FILENAME = "checksums.txt";
const CHECKSUM_PATTERN = /^([a-f0-9]{64})\s+\*?(.+)$/i;
const MAX_REDIRECTS = 3;
const LOCAL_HTTP_HOSTS = new Set(["127.0.0.1", "localhost"]);

function resolveDownloadClient(parsedUrl) {
  const isHttps = parsedUrl.protocol === "https:";

  if (isHttps) return https;

  const isHttp = parsedUrl.protocol === "http:";
  const isLocalHost = LOCAL_HTTP_HOSTS.has(parsedUrl.hostname);
  const isAllowedLocalHttp = isHttp && isLocalHost;

  if (isAllowedLocalHttp) return http;

  return null;
}

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

async function ensureManagedBinary(params) {
  const {
    shouldSkip,
    platformAsset,
    baseUrl,
    managedBinaryPath,
    version,
    platform,
  } = params;

  if (shouldSkip) return null;

  const hasPlatform = platformAsset != null;
  const hasUrl = baseUrl != null;
  const hasPath = managedBinaryPath != null;
  const hasAllRequirements = hasPlatform && hasUrl && hasPath;

  if (!hasAllRequirements) return null;

  const alreadyExists = fs.existsSync(managedBinaryPath);

  if (alreadyExists) return managedBinaryPath;

  const targetDir = path.dirname(managedBinaryPath);

  fs.mkdirSync(targetDir, { recursive: true });

  const checksumsUrl = `${baseUrl}/${CHECKSUM_FILENAME}`;
  const assetUrl = `${baseUrl}/${platformAsset.assetName}`;

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

  const tempPath = `${managedBinaryPath}.tmp`;

  fs.writeFileSync(tempPath, binaryData);

  const isNotWindows = platform !== "win32";

  if (isNotWindows) fs.chmodSync(tempPath, 0o700);

  fs.renameSync(tempPath, managedBinaryPath);

  return managedBinaryPath;
}

module.exports = {
  ensureManagedBinary,
};
