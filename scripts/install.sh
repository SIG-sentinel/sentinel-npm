#!/bin/sh

set -eu

VERSION="latest"
REPO="SIG-sentinel/sentinel-npm"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BASE_URL="${SENTINEL_RELEASE_BASE_URL:-}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --repo)
      REPO="$2"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    --base-url)
      BASE_URL="$2"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

# Validate REPO to prevent URL injection (must be owner/repo)
case "$REPO" in
  *[!A-Za-z0-9._/-]*)
    echo "invalid repository format: $REPO" >&2
    exit 1
    ;;
  */*/*)  # more than one slash
    echo "invalid repository format: $REPO" >&2
    exit 1
    ;;
esac

platform=$(uname -s)
arch=$(uname -m)

case "$platform:$arch" in
  Linux:x86_64)
    asset_name="sentinel-linux-x64"
    binary_name="sentinel"
    ;;
  Darwin:x86_64)
    asset_name="sentinel-darwin-x64"
    binary_name="sentinel"
    ;;
  Darwin:arm64)
    asset_name="sentinel-darwin-arm64"
    binary_name="sentinel"
    ;;
  *)
    echo "unsupported platform: $platform/$arch" >&2
    exit 1
    ;;
esac

if [ -z "$BASE_URL" ]; then
  if [ "$VERSION" = "latest" ]; then
    base_url="https://github.com/$REPO/releases/latest/download"
  else
    case "$VERSION" in
      v*) ;;
      *) VERSION="v$VERSION" ;;
    esac
    base_url="https://github.com/$REPO/releases/download/$VERSION"
  fi
else
  base_url="$BASE_URL"
fi

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

binary_path="$tmp_dir/$asset_name"
checksums_path="$tmp_dir/checksums.txt"

curl -fsSL "$base_url/$asset_name" -o "$binary_path"
curl -fsSL "$base_url/checksums.txt" -o "$checksums_path"

expected_checksum=$(awk -v target="$asset_name" '$2 == target { print $1 }' "$checksums_path")
if [ -z "$expected_checksum" ]; then
  echo "checksum not found for $asset_name" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  actual_checksum=$(sha256sum "$binary_path" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  actual_checksum=$(shasum -a 256 "$binary_path" | awk '{print $1}')
else
  echo "sha256 tool not found (need sha256sum or shasum)" >&2
  exit 1
fi

if [ "$expected_checksum" != "$actual_checksum" ]; then
  echo "checksum mismatch for $asset_name" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
install "$binary_path" "$INSTALL_DIR/$binary_name"

echo "sentinel installed to $INSTALL_DIR/$binary_name"
