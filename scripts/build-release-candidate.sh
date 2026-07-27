#!/usr/bin/env bash
set -euo pipefail

readonly VERSION="1.0.0"
readonly TARGET="xtensa-esp32s3-none-elf"
readonly FIRMWARE="target/$TARGET/release/pokeviewer-firmware"
readonly CLI="target/release/pokeviewerctl"

if [[ $# -ne 1 ]]; then
  echo "usage: $0 OUTPUT_DIRECTORY" >&2
  exit 2
fi

output_dir=$1
if [[ -e "$output_dir" ]]; then
  echo "output already exists: $output_dir" >&2
  exit 1
fi

for tool in cargo espflash git gzip sha256sum stat strings tar; do
  if ! command -v "$tool" >/dev/null; then
    echo "required tool is unavailable: $tool" >&2
    exit 1
  fi
done

package_id=$(cargo pkgid --locked -p pokeviewer-core)
if [[ ${package_id##*#} != "$VERSION" ]]; then
  echo "workspace version does not match release version $VERSION" >&2
  exit 1
fi

commit=$(git rev-parse --verify HEAD)
source_date_epoch=${SOURCE_DATE_EPOCH:-$(git show -s --format=%ct "$commit")}
export SOURCE_DATE_EPOCH="$source_date_epoch"
export ESPFLASH_SKIP_UPDATE_CHECK=true
export CARGO_PROFILE_RELEASE_DEBUG=0
export CARGO_PROFILE_RELEASE_STRIP=symbols
cargo_home=${CARGO_HOME:-"$HOME/.cargo"}
cargo_home=$(cd "$cargo_home" && pwd -P)
xtensa_sysroot=$(rustup run esp-1.95.0.0 rustc --print sysroot)
remap_flags="--remap-path-prefix=$PWD=/src/pokeviewer"
remap_flags+=" --remap-path-prefix=$cargo_home=/src/cargo"
remap_flags+=" --remap-path-prefix=$xtensa_sysroot=/src/xtensa"
export CARGO_TARGET_XTENSA_ESP32S3_NONE_ELF_RUSTFLAGS="$remap_flags"

pack_before=$(sha256sum content/generated/pokeviewer-v1.pack)
manifest_before=$(sha256sum content/generated/pokeviewer-v1.json)
cargo xtask content-build
if [[ "$pack_before" != "$(sha256sum content/generated/pokeviewer-v1.pack)" ||
  "$manifest_before" != "$(sha256sum content/generated/pokeviewer-v1.json)" ]]; then
  echo "committed content does not match a clean offline rebuild" >&2
  exit 1
fi
cargo xtask firmware-build
scripts/check-firmware-artifact.sh "$FIRMWARE" "$output_dir-firmware-check"
RUSTFLAGS="$remap_flags" cargo build --release --locked -p pokeviewerctl

mkdir -p target
work_dir=$(mktemp -d "$PWD/target/release-work.XXXXXX")
trap 'rm -rf "$work_dir" "$output_dir-firmware-check"' EXIT
bundle_name="pokeviewer-v$VERSION"
bundle_dir="$work_dir/$bundle_name"
mkdir -p "$bundle_dir"

firmware_bin="pokeviewer-v$VERSION-esp32s3-v2.bin"
cli_bin="pokeviewerctl-v$VERSION-x86_64-unknown-linux-gnu"

espflash save-image \
  --chip esp32s3 \
  --merge \
  --skip-padding \
  "$FIRMWARE" \
  "$bundle_dir/$firmware_bin"

cp "$CLI" "$bundle_dir/$cli_bin"
cp content/generated/pokeviewer-v1.pack "$bundle_dir/"
cp content/generated/pokeviewer-v1.json "$bundle_dir/content-manifest.json"
cp release/FLASHING.md "$bundle_dir/"
cp release/RELEASE-NOTES.md "$bundle_dir/"
cp release/CANDIDATE-STATUS.md "$bundle_dir/"
cp docs/user-guide.md "$bundle_dir/USER-GUIDE.md"
cp docs/safety.md "$bundle_dir/SAFETY.md"
cp docs/troubleshooting.md "$bundle_dir/TROUBLESHOOTING.md"
cp docs/release-verification.md "$bundle_dir/RELEASE-VERIFICATION.md"
cp LICENSE THIRD_PARTY_NOTICES.md "$bundle_dir/"
chmod 0644 "$bundle_dir"/*
chmod 0755 "$bundle_dir/$cli_bin"

if strings "$bundle_dir/$firmware_bin" "$bundle_dir/$cli_bin" | awk '
  /\/home\// || /\/Users\// || /[A-Za-z]:\\Users\\/ { found = 1 }
  END { exit found ? 0 : 1 }
'; then
  echo "distributed binary contains a machine-specific path" >&2
  exit 1
fi

content_hash=$(sha256sum "$bundle_dir/pokeviewer-v1.pack")
content_hash=${content_hash%% *}
cat >"$bundle_dir/BUILD-METADATA.txt" <<EOF
product_version=$VERSION
source_commit=$commit
source_date_epoch=$source_date_epoch
board=waveshare-esp32-s3-epaper-1.54-en-v2-non-touch
firmware_target=$TARGET
firmware_flash_offset=0x0
cli_target=x86_64-unknown-linux-gnu
protocol_version=1
content_format_version=1
content_revision=1
schedule_version=1
content_pack_sha256=$content_hash
rust_host=$(rustc --version)
rust_xtensa=$(rustup run esp-1.95.0.0 rustc --version)
espflash=$(espflash --version)
EOF

payloads=(
  "$firmware_bin"
  "$cli_bin"
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
)

{
  echo "Pokeviewer release candidate manifest"
  echo "version=$VERSION"
  echo "source_commit=$commit"
  echo "file_count=${#payloads[@]}"
  echo
  echo "sha256 size_bytes filename"
  for file in "${payloads[@]}"; do
    hash=$(sha256sum "$bundle_dir/$file")
    hash=${hash%% *}
    size=$(stat --format=%s "$bundle_dir/$file")
    printf '%s %s %s\n' "$hash" "$size" "$file"
  done
} >"$bundle_dir/MANIFEST.txt"

(
  cd "$bundle_dir"
  sha256sum "${payloads[@]}" MANIFEST.txt >SHA256SUMS
)

mkdir -p "$output_dir"
archive="$output_dir/$bundle_name.tar.gz"
tar \
  --sort=name \
  --format=ustar \
  --mtime="@$source_date_epoch" \
  --owner=0 \
  --group=0 \
  --numeric-owner \
  -C "$work_dir" \
  -cf - \
  "$bundle_name" \
  | gzip -n >"$archive"
(
  cd "$output_dir"
  sha256sum "$bundle_name.tar.gz" >"$bundle_name.tar.gz.sha256"
)

scripts/verify-release-candidate.sh "$archive"
echo "created $archive"
