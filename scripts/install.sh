
set -eu

VERSION=""
REPO="SIG-sentinel/sentinel-npm"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BASE_URL="${SENTINEL_RELEASE_BASE_URL:-}"

require_arg() {
  option_name="$1"

  if [ "$#" -lt 2 ] || [ -z "$2" ]; then
    echo "missing value for $option_name" >&2
    exit 1
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      require_arg "$@"
      VERSION="$2"
      shift 2
      ;;
    --repo)
      require_arg "$@"
      REPO="$2"
      shift 2
      ;;
    --install-dir)
      require_arg "$@"
      INSTALL_DIR="$2"
      shift 2
      ;;
    --base-url)
      require_arg "$@"
      BASE_URL="$2"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

case "$REPO" in
  [A-Za-z0-9._-]*/[A-Za-z0-9._-]*)
    ;;
  *)
    echo "invalid repository format: $REPO" >&2
    exit 1
    ;;
esac

if [ -z "$BASE_URL" ] && [ -z "$VERSION" ]; then
  echo "--version is required (example: --version 1.1.1)" >&2
  exit 1
fi

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
  case "$VERSION" in
    v*) ;;
    *) VERSION="v$VERSION" ;;
  esac
  base_url="https://github.com/$REPO/releases/download/$VERSION"
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

SENTINEL_PUBLIC_KEY='-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE2golH9pruEs3kBIQzfv2QbO0pnYc
M+HG/ijCK2FXoIOe6Xp3/WqWzighRowDBKxy0Y7duM03hVsRTcRcFvgHIA==
-----END PUBLIC KEY-----'

sig_path="$tmp_dir/checksums.txt.sig"
if ! command -v openssl >/dev/null 2>&1; then
  echo "openssl not found (required for signature verification)" >&2
  exit 1
fi

if ! curl -fsSL "$base_url/checksums.txt.sig" -o "$sig_path"; then
  echo "signature file not found: checksums.txt.sig" >&2
  exit 1
fi

pub_path="$tmp_dir/sentinel_pub.pem"
printf '%s\n' "$SENTINEL_PUBLIC_KEY" > "$pub_path"
if ! openssl dgst -sha256 -verify "$pub_path" -signature "$sig_path" "$checksums_path" >/dev/null 2>&1; then
  echo "signature verification failed for checksums.txt" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"
install "$binary_path" "$INSTALL_DIR/$binary_name"

echo "sentinel installed to $INSTALL_DIR/$binary_name"
