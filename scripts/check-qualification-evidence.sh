#!/usr/bin/env bash
set -euo pipefail

evidence_dir=${1:?usage: check-qualification-evidence.sh EVIDENCE_DIR}
required=(
  metadata.env checklist.md measurements.csv capacity.txt
  seven-day.csv photos.csv
)

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
if ! grep -Eq '^schema_version=1$' "$evidence_dir/metadata.env"; then
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
BEGIN { number = "^[0-9]+([.][0-9]+)?$" }
NR == 1 {
  if ($0 != "phase,current_ma,duration_seconds,instrument_range,sample_rate_hz,status") exit 1
  next
}
NF != 6 || $1 !~ /^(refresh|awake_idle|deep_sleep)$/ ||
  $2 !~ number || $3 !~ number || $4 == "" || $5 !~ number ||
  $6 != "PASS" { exit 1 }
{ seen[$1]++ }
$1 == "deep_sleep" && ($2 > 0.500 || $3 < 60) { exit 1 }
$1 == "refresh" && $3 > 10 { exit 1 }
$1 == "refresh" || $1 == "awake_idle" { active += $3 }
END {
  if (NR != 4 || seen["refresh"] != 1 || seen["awake_idle"] != 1 ||
      seen["deep_sleep"] != 1 || active > 30) exit 1
}
' "$evidence_dir/measurements.csv" || {
  echo "measurement rows or release thresholds failed" >&2
  exit 1
}
awk -F= '
BEGIN { number = "^[0-9]+([.][0-9]+)?$" }
$1 == "minimum_72h_mAh" && $2 ~ number { minimum = $2 + 0; minimum_seen++ }
$1 == "intended_cell_rated_mAh" && $2 ~ number { rated = $2 + 0; rated_seen++ }
$1 == "usable_capacity_fraction" && $2 ~ number { usable = $2 + 0; usable_seen++ }
$1 == "status" { status = $2; status_seen++ }
END {
  if (minimum_seen != 1 || rated_seen != 1 || usable_seen != 1 ||
      status_seen != 1 || minimum <= 0 || usable <= 0 || usable > 1 ||
      rated * usable < minimum || status != "PASS") exit 1
}
' "$evidence_dir/capacity.txt" || {
  echo "battery capacity evidence failed" >&2
  exit 1
}
awk -F, '
NR == 1 {
  if ($0 != "day,date,weekday,dex_id,name,framebuffer_crc32,status") exit 1
  next
}
NF != 7 || $1 != NR - 1 ||
  $2 !~ /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/ ||
  $3 !~ /^(Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday)$/ ||
  $4 !~ /^[0-9]+$/ || $4 < 1 || $4 > 151 || $5 == "" ||
  length($6) != 8 || $6 !~ /^[0-9a-f]+$/ || $7 != "PASS" { exit 1 }
END { if (NR != 8) exit 1 }
' "$evidence_dir/seven-day.csv" || {
  echo "seven-day schedule evidence is incomplete" >&2
  exit 1
}
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
