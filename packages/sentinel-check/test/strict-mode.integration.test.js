const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const { test } = require("node:test");

function writeExecutable(filePath, contents) {
  fs.writeFileSync(filePath, contents, "utf8");
  fs.chmodSync(filePath, 0o755);
}

test("strict mode blocks PATH fallback when managed binary is unavailable", () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "sentinel-wrapper-"));
  const markerPath = path.join(tmpDir, "marker.txt");
  const fakeSentinel = path.join(tmpDir, "sentinel");
  const sentinelJs = path.resolve(__dirname, "../bin/sentinel.js");

  writeExecutable(
    fakeSentinel,
    `#!/usr/bin/env sh\necho fake > "${markerPath}"\nexit 0\n`
  );

  const result = spawnSync(process.execPath, [sentinelJs, "--version"], {
    cwd: tmpDir,
    env: {
      ...process.env,
      SENTINEL_SKIP_DOWNLOAD: "1",
      PATH: `${tmpDir}${path.delimiter}${process.env.PATH || ""}`,
    },
    encoding: "utf8",
  });

  assert.equal(result.status, 1);
  assert.match(result.stderr, /strict mode blocks unverified fallback binaries/i);
  assert.equal(fs.existsSync(markerPath), false);
});

test("unsafe override allows unverified fallback only when explicitly enabled", () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "sentinel-wrapper-"));
  const markerPath = path.join(tmpDir, "marker.txt");
  const fakeSentinel = path.join(tmpDir, "sentinel");
  const sentinelJs = path.resolve(__dirname, "../bin/sentinel.js");

  writeExecutable(
    fakeSentinel,
    `#!/usr/bin/env sh\necho allowed > "${markerPath}"\nexit 0\n`
  );

  const result = spawnSync(process.execPath, [sentinelJs, "--version"], {
    cwd: tmpDir,
    env: {
      ...process.env,
      SENTINEL_SKIP_DOWNLOAD: "1",
      SENTINEL_ALLOW_UNVERIFIED_FALLBACK: "1",
      SENTINEL_BIN: fakeSentinel,
      PATH: `${tmpDir}${path.delimiter}${process.env.PATH || ""}`,
    },
    encoding: "utf8",
  });

  assert.equal(result.status, 0);
  assert.equal(fs.existsSync(markerPath), true);
});