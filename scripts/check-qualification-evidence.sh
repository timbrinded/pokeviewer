#!/usr/bin/env bash
set -euo pipefail

evidence_dir=${1:?usage: check-qualification-evidence.sh EVIDENCE_DIR}
required=(metadata.env checklist.md measurements.csv capacity.txt)

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
for field in firmware_commit cli_commit; do
  if ! grep -Eq "^${field}=[0-9a-f]{40}$" "$evidence_dir/metadata.env"; then
    echo "$field must be a full Git commit" >&2
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
NR == 1 { next }
NF != 6 || $6 != "PASS" { exit 1 }
$1 == "deep_sleep" && $2 > 0.500 { exit 1 }
$1 == "refresh" && $3 > 10 { exit 1 }
$1 == "refresh" || $1 == "awake_idle" { active += $3 }
END { if (NR != 4 || active > 30) exit 1 }
' "$evidence_dir/measurements.csv" || {
  echo "measurement rows or release thresholds failed" >&2
  exit 1
}
awk -F= '
$1 == "minimum_72h_mAh" { minimum = $2 + 0 }
$1 == "intended_cell_rated_mAh" { rated = $2 + 0 }
$1 == "usable_capacity_fraction" { usable = $2 + 0 }
$1 == "status" { status = $2 }
END {
  if (minimum <= 0 || rated < minimum || usable <= 0 || usable > 1 ||
      status != "PASS") exit 1
}
' "$evidence_dir/capacity.txt" || {
  echo "battery capacity evidence failed" >&2
  exit 1
}
if grep -EIRq \
  '(/home/|/Users/|tty(ACM|USB)[0-9]+|MAC[=:]|serial[=:]|ssid[=:]|password[=:])' \
  "$evidence_dir"; then
  echo "qualification evidence contains a forbidden private identifier" >&2
  exit 1
fi

echo "qualification evidence is complete and sanitized"
