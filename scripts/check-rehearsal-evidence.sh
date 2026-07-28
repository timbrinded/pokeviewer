#!/usr/bin/env bash
set -euo pipefail

readonly BOARD="waveshare-esp32-s3-epaper-1.54-en-v2-non-touch"

if [[ $# -ne 2 ]]; then
  echo "usage: $0 EVIDENCE_DIRECTORY CANDIDATE_ARCHIVE" >&2
  exit 2
fi

evidence_dir=$1
archive=$2
values="$evidence_dir/rehearsal.env"
transcript="$evidence_dir/terminal.txt"
checklist="$evidence_dir/checklist.md"
for tool in awk file sha256sum tar; do
  if ! command -v "$tool" >/dev/null; then
    echo "required tool is unavailable: $tool" >&2
    exit 1
  fi
done

required_files=(
  "$values"
  "$transcript"
  "$checklist"
  "$evidence_dir/setup.png"
  "$evidence_dir/daily-card.png"
  "$evidence_dir/scheduled-refresh.png"
  "$evidence_dir/invalid-rtc.png"
  "$evidence_dir/failure-recovery.png"
)
for file in "${required_files[@]}"; do
  if [[ ! -s "$file" ]]; then
    echo "missing or empty rehearsal evidence: $file" >&2
    exit 1
  fi
done
for image in "$evidence_dir"/*.png; do
  mime=$(file --brief --mime-type "$image")
  if [[ "$mime" != "image/png" && "$mime" != "image/jpeg" ]]; then
    echo "rehearsal photograph has unsupported content: $image" >&2
    exit 1
  fi
done

scripts/verify-release-candidate.sh "$archive"

value() {
  local key=$1
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2) }' \
    "$values"
}

commit=$(value candidate_commit)
if [[ ! "$commit" =~ ^[0-9a-f]{40}$ ]]; then
  echo "candidate_commit must be a full lowercase Git commit" >&2
  exit 1
fi

for key in archive_sha256 firmware_sha256 cli_sha256 content_pack_sha256; do
  if [[ ! "$(value "$key")" =~ ^[0-9a-f]{64}$ ]]; then
    echo "$key must be a lowercase SHA-256 value" >&2
    exit 1
  fi
done

if [[ "$(value board)" != "$BOARD" ]]; then
  echo "rehearsal board does not match the supported V2 contract" >&2
  exit 1
fi

pass_keys=(
  outer_checksum
  inner_checksums
  clean_host
  erased_flash
  firmware_flash
  rtc_readback
  daily_card
  deep_sleep
  scheduled_refresh
  invalid_rtc_recovery
  panel_failure_recovery
  documentation_only
)
for key in "${pass_keys[@]}"; do
  if [[ "$(value "$key")" != "PASS" ]]; then
    echo "$key is not PASS" >&2
    exit 1
  fi
done
if [[ "$(value unresolved_blockers)" != "NONE" ]]; then
  echo "rehearsal has unresolved blockers" >&2
  exit 1
fi

archive_hash=$(sha256sum "$archive")
archive_hash=${archive_hash%% *}
if [[ "$(value archive_sha256)" != "$archive_hash" ]]; then
  echo "recorded archive hash does not match candidate" >&2
  exit 1
fi

metadata=$(tar -xOf "$archive" \
  pokeviewer-v1.0.0/BUILD-METADATA.txt)
metadata_commit=$(awk -F= '$1 == "source_commit" { print $2 }' <<<"$metadata")
if [[ "$commit" != "$metadata_commit" ]]; then
  echo "recorded commit does not match candidate metadata" >&2
  exit 1
fi

sums=$(tar -xOf "$archive" pokeviewer-v1.0.0/SHA256SUMS)
firmware_hash=$(awk '$2 == "pokeviewer-v1.0.0-esp32s3-v2.bin" { print $1 }' \
  <<<"$sums")
cli_hash=$(awk \
  '$2 == "pokeviewerctl-v1.0.0-x86_64-unknown-linux-gnu" { print $1 }' \
  <<<"$sums")
pack_hash=$(awk '$2 == "pokeviewer-v1.pack" { print $1 }' <<<"$sums")
if [[ "$(value firmware_sha256)" != "$firmware_hash" ||
  "$(value cli_sha256)" != "$cli_hash" ||
  "$(value content_pack_sha256)" != "$pack_hash" ]]; then
  echo "recorded payload hash does not match candidate" >&2
  exit 1
fi

if awk '
  /\/dev\/(tty|serial)/ ||
  /\/home\// ||
  /\/Users\// ||
  /[0-9A-Fa-f][0-9A-Fa-f]:[0-9A-Fa-f][0-9A-Fa-f]:[0-9A-Fa-f][0-9A-Fa-f]:[0-9A-Fa-f][0-9A-Fa-f]:[0-9A-Fa-f][0-9A-Fa-f]:[0-9A-Fa-f][0-9A-Fa-f]/ {
    private = 1
  }
  END { exit private ? 0 : 1 }
' "$transcript"; then
  echo "terminal transcript contains a private machine or device identifier" >&2
  exit 1
fi

if ! awk '
  /^- \[x\]/ { complete += 1 }
  END { exit complete == 15 ? 0 : 1 }
' "$checklist"; then
  echo "rehearsal checklist is incomplete" >&2
  exit 1
fi

echo "clean-host rehearsal evidence is complete"
