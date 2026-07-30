#!/usr/bin/env bash
set -euo pipefail

evidence_dir=${1:?usage: check-qualification-evidence.sh EVIDENCE_DIR}
required=(metadata.env checklist.md seven-day.csv photos.csv)
for tool in cargo cmp sed; do
  if ! command -v "$tool" >/dev/null; then
    echo "required tool is unavailable: $tool" >&2
    exit 1
  fi
done

for filename in "${required[@]}"; do
  if [[ ! -s "$evidence_dir/$filename" ]]; then
    echo "missing qualification evidence: $filename" >&2
    exit 1
  fi
done

if ! grep -Eq '^qualification_status=PASS$' "$evidence_dir/metadata.env"; then
  echo "qualification_status must be PASS" >&2
  exit 1
fi
if ! grep -Eq '^schema_version=2$' "$evidence_dir/metadata.env"; then
  echo "unsupported qualification evidence schema" >&2
  exit 1
fi
for field in firmware_commit cli_commit; do
  if ! grep -Eq "^${field}=[0-9a-f]{40}$" "$evidence_dir/metadata.env"; then
    echo "$field must be a full Git commit" >&2
    exit 1
  fi
done
for field in firmware_sha256 content_pack_sha256; do
  if ! grep -Eq "^${field}=[0-9a-f]{64}$" "$evidence_dir/metadata.env"; then
    echo "$field must be a SHA-256 digest" >&2
    exit 1
  fi
done
if ! grep -Eq '^board_revision=V2$' "$evidence_dir/metadata.env"; then
  echo "board_revision must be V2" >&2
  exit 1
fi
if grep -EIRq 'PENDING|REPLACE_' "$evidence_dir"; then
  echo "qualification evidence still contains a placeholder" >&2
  exit 1
fi
if grep -Eq '\[ \]|FAIL' "$evidence_dir/checklist.md"; then
  echo "qualification checklist is incomplete or failed" >&2
  exit 1
fi
awk -F, '
NR == 1 {
  if ($0 != "day,date,weekday,dex_id,name,battery_percent,framebuffer_crc32,status") exit 1
  next
}
NF != 8 || $1 != NR - 1 ||
  $2 !~ /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/ ||
  $3 !~ /^(Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday)$/ ||
  $4 !~ /^[0-9]+$/ || $4 < 1 || $4 > 151 || $5 == "" ||
  $6 !~ /^(0|10|20|30|40|50|60|70|80|90|100)$/ ||
  length($7) != 8 || $7 !~ /^[0-9a-f]+$/ || $8 != "PASS" { exit 1 }
END { if (NR != 8) exit 1 }
' "$evidence_dir/seven-day.csv" || {
  echo "seven-day schedule evidence is incomplete" >&2
  exit 1
}
mkdir -p target
expected_schedule=$(mktemp "$PWD/target/qualification-schedule.XXXXXX")
trap 'rm -f -- "$expected_schedule"' EXIT
start_date=$(awk -F, 'NR == 2 { print $2 }' "$evidence_dir/seven-day.csv")
battery_percentages=$(awk -F, 'NR > 1 { values = values separator $6; separator = "," } END { print values }' "$evidence_dir/seven-day.csv")
cargo run --quiet --locked --package xtask -- \
  qualification-schedule "$start_date" "$expected_schedule" "$battery_percentages"
sed -i 's/,PENDING$/,PASS/' "$expected_schedule"
if ! cmp --silent "$expected_schedule" "$evidence_dir/seven-day.csv"; then
  echo "seven-day evidence does not match the deterministic schedule" >&2
  exit 1
fi
awk -F, '
NR == 1 {
  if ($0 != "day,phase,filename,sha256,status") exit 1
  next
}
NF != 5 || $1 !~ /^[1-7]$/ || $2 !~ /^(before|after)$/ ||
  $3 == "" || length($4) != 64 || $4 !~ /^[0-9a-f]+$/ ||
  $5 != "PASS" { exit 1 }
{ seen[$1 "," $2]++ }
END {
  if (NR != 15) exit 1
  for (day = 1; day <= 7; day++) {
    if (seen[day ",before"] != 1 || seen[day ",after"] != 1) exit 1
  }
}
' "$evidence_dir/photos.csv" || {
  echo "seven-day photo manifest is incomplete" >&2
  exit 1
}
if grep -EIRq \
  '(/home/|/Users/|tty(ACM|USB)[0-9]+|MAC[=:]|serial[=:]|ssid[=:]|password[=:])' \
  "$evidence_dir"; then
  echo "qualification evidence contains a forbidden private identifier" >&2
  exit 1
fi

echo "qualification evidence is complete and sanitized"
