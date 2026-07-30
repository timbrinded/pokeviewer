#!/usr/bin/env bash
set -euo pipefail

readonly VERSION="1.1.0"
readonly BUNDLE="pokeviewer-v$VERSION"

if [[ $# -ne 1 ]]; then
  echo "usage: $0 ARCHIVE" >&2
  exit 2
fi

archive=$1
checksum_file="$archive.sha256"
if [[ ! -f "$archive" || ! -f "$checksum_file" ]]; then
  echo "archive and adjacent .sha256 file are required" >&2
  exit 1
fi
for tool in awk jq sha256sum stat tar; do
  if ! command -v "$tool" >/dev/null; then
    echo "required tool is unavailable: $tool" >&2
    exit 1
  fi
done

(
  cd "$(dirname "$archive")"
  sha256sum --check "$(basename "$checksum_file")"
)

if ! tar -tzf "$archive" | awk '
  /^\// || /(^|\/)\.\.($|\/)/ { exit 1 }
'; then
  echo "archive contains an unsafe path" >&2
  exit 1
fi

mkdir -p target
work_dir=$(mktemp -d "$PWD/target/verify-release.XXXXXX")
trap 'rm -rf "$work_dir"' EXIT
tar -xzf "$archive" -C "$work_dir"
bundle_dir="$work_dir/$BUNDLE"

expected=(
  "pokeviewer-v$VERSION-esp32s3-v2.bin"
  "pokeviewerctl-v$VERSION-x86_64-unknown-linux-gnu"
  "pokeviewer-v1.pack"
  "content-manifest.json"
  "BUILD-METADATA.txt"
  "FLASHING.md"
  "RELEASE-NOTES.md"
  "CANDIDATE-STATUS.md"
  "USER-GUIDE.md"
  "SAFETY.md"
  "TROUBLESHOOTING.md"
  "RELEASE-VERIFICATION.md"
  "LICENSE"
  "THIRD_PARTY_NOTICES.md"
  "MANIFEST.txt"
  "SHA256SUMS"
)

actual_count=$(printf '%s\n' "$bundle_dir"/* | wc -l)
if [[ "$actual_count" -ne "${#expected[@]}" ]]; then
  echo "candidate contains an unexpected number of files" >&2
  exit 1
fi
for file in "${expected[@]}"; do
  if [[ ! -f "$bundle_dir/$file" ]]; then
    echo "candidate is missing $file" >&2
    exit 1
  fi
done

(
  cd "$bundle_dir"
  sha256sum --check SHA256SUMS
)

reported_version=$("$bundle_dir/pokeviewerctl-v$VERSION-x86_64-unknown-linux-gnu" --version)
if [[ "$reported_version" != "pokeviewerctl $VERSION" ]]; then
  echo "CLI version does not match candidate version" >&2
  exit 1
fi

metadata_version=$(awk -F= '$1 == "product_version" { print $2 }' \
  "$bundle_dir/BUILD-METADATA.txt")
if [[ "$metadata_version" != "$VERSION" ]]; then
  echo "build metadata version does not match candidate version" >&2
  exit 1
fi

metadata_value() {
  local key=$1
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2) }' \
    "$bundle_dir/BUILD-METADATA.txt"
}

if [[ ! "$(metadata_value source_commit)" =~ ^[0-9a-f]{40}$ ||
  "$(metadata_value board)" != "waveshare-esp32-s3-epaper-1.54-en-v2-non-touch" ||
  "$(metadata_value firmware_target)" != "xtensa-esp32s3-none-elf" ||
  "$(metadata_value firmware_flash_offset)" != "0x0" ||
  "$(metadata_value cli_target)" != "x86_64-unknown-linux-gnu" ||
  "$(metadata_value protocol_version)" != "1" ]]; then
  echo "build metadata does not match the v1 compatibility contract" >&2
  exit 1
fi

manifest="$bundle_dir/content-manifest.json"
if [[ "$(metadata_value content_format_version)" != "$(jq -er '.format_version' "$manifest")" ||
  "$(metadata_value content_revision)" != "$(jq -er '.content_revision' "$manifest")" ||
  "$(metadata_value schedule_version)" != "$(jq -er '.schedule_version' "$manifest")" ]]; then
  echo "build metadata content versions do not match the content manifest" >&2
  exit 1
fi

pack="$bundle_dir/pokeviewer-v1.pack"
pack_hash=$(sha256sum "$pack")
pack_hash=${pack_hash%% *}
if [[ "$(metadata_value content_pack_sha256)" != "$pack_hash" ||
  "$(jq -er '.pack_sha256' "$manifest")" != "$pack_hash" ||
  "$(jq -er '.pack_length' "$manifest")" != "$(stat --format=%s "$pack")" ||
  "$(jq -er '.pack_path' "$manifest")" != "pokeviewer-v1.pack" ]]; then
  echo "packaged content does not match its metadata or manifest" >&2
  exit 1
fi

echo "verified $archive"
