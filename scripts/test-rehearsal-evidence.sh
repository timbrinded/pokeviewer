#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 CANDIDATE_ARCHIVE" >&2
  exit 2
fi

archive=$1
validator=scripts/check-rehearsal-evidence.sh
mkdir -p target
work_dir=$(mktemp -d "$PWD/target/rehearsal-test.XXXXXX")
trap 'rm -rf -- "$work_dir"' EXIT

cp docs/evidence/setup-screen/invalid-rtc-setup.png "$work_dir/setup.png"
cp "$work_dir/setup.png" "$work_dir/daily-card.png"
cp "$work_dir/setup.png" "$work_dir/scheduled-refresh.png"
cp "$work_dir/setup.png" "$work_dir/invalid-rtc.png"
cp "$work_dir/setup.png" "$work_dir/failure-recovery.png"
sed 's/^- \[ \]/- [x]/' \
  docs/evidence/rehearsal-template/checklist.md >"$work_dir/checklist.md"
printf '%s\n' \
  'Sanitized synthetic validator test.' \
  'Commands use DEVICE and contain no private identifiers.' \
  >"$work_dir/terminal.txt"

metadata=$(tar -xOf "$archive" pokeviewer-v1.1.0/BUILD-METADATA.txt)
sums=$(tar -xOf "$archive" pokeviewer-v1.1.0/SHA256SUMS)
value_from_metadata() {
  local key=$1
  awk -F= -v key="$key" '$1 == key { print substr($0, length(key) + 2) }' \
    <<<"$metadata"
}
hash_from_sums() {
  local filename=$1
  awk -v filename="$filename" '$2 == filename { print $1 }' <<<"$sums"
}
file_hash() {
  local filename=$1
  local hash
  hash=$(sha256sum "$work_dir/$filename")
  printf '%s' "${hash%% *}"
}

archive_hash=$(sha256sum "$archive")
archive_hash=${archive_hash%% *}
cat >"$work_dir/rehearsal.env" <<EOF
candidate_commit=$(value_from_metadata source_commit)
archive_sha256=$archive_hash
firmware_sha256=$(hash_from_sums pokeviewer-v1.1.0-esp32s3-v2.bin)
cli_sha256=$(hash_from_sums pokeviewerctl-v1.1.0-x86_64-unknown-linux-gnu)
content_pack_sha256=$(hash_from_sums pokeviewer-v1.pack)
setup_photo_sha256=$(file_hash setup.png)
daily_card_photo_sha256=$(file_hash daily-card.png)
scheduled_refresh_photo_sha256=$(file_hash scheduled-refresh.png)
invalid_rtc_photo_sha256=$(file_hash invalid-rtc.png)
failure_recovery_photo_sha256=$(file_hash failure-recovery.png)
board=waveshare-esp32-s3-epaper-1.54-en-v2-non-touch
outer_checksum=PASS
inner_checksums=PASS
clean_host=PASS
erased_flash=PASS
firmware_flash=PASS
rtc_readback=PASS
daily_card=PASS
deep_sleep=PASS
scheduled_refresh=PASS
pwr_short_press=PASS
parent_session=PASS
storage_mode=PASS
battery_display=PASS
invalid_rtc_recovery=PASS
panel_failure_recovery=PASS
documentation_only=PASS
unresolved_blockers=NONE
EOF

"$validator" "$work_dir" "$archive"

expect_failure() {
  local description=$1
  if "$validator" "$work_dir" "$archive" >/dev/null 2>&1; then
    echo "invalid rehearsal evidence passed: $description" >&2
    exit 1
  fi
}

sed -i 's/deep_sleep=PASS/deep_sleep=FAIL/' "$work_dir/rehearsal.env"
expect_failure 'failed deep-sleep result'

sed -i 's/deep_sleep=FAIL/deep_sleep=PASS/' "$work_dir/rehearsal.env"
printf '%s\n' 'device=/dev/ttyACM0' >>"$work_dir/terminal.txt"
expect_failure 'private serial path'

sed -i '$d' "$work_dir/terminal.txt"
sed -i '0,/\[x\]/s//[ ]/' "$work_dir/checklist.md"
expect_failure 'incomplete checklist'

sed 's/^- \[ \]/- [x]/' \
  docs/evidence/rehearsal-template/checklist.md >"$work_dir/checklist.md"
printf 'tampered' >>"$work_dir/setup.png"
expect_failure 'changed photograph'
