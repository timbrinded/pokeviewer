#!/usr/bin/env bash
set -euo pipefail

calculator=scripts/calculate-battery-capacity.sh
result=$("$calculator" 0.100 80 2 120 5 0.80)

grep -Fxq 'sleep_hours=71.994167' <<<"$result"
grep -Fxq 'active_mAh=0.133333' <<<"$result"
grep -Fxq 'refresh_mAh=0.500000' <<<"$result"
grep -Fxq 'sleep_mAh=7.199417' <<<"$result"
grep -Fxq 'minimum_72h_mAh=9.791' <<<"$result"

for invalid in \
  'text 80 2 120 5 0.80' \
  '-0.1 80 2 120 5 0.80' \
  '0.1 80 2 120 5 0' \
  '0.1 80 2 120 5 1.01' \
  '0.1 80 259200 120 5 0.80'
do
  read -r -a arguments <<<"$invalid"
  if "$calculator" "${arguments[@]}" >/dev/null 2>&1; then
    echo "invalid capacity input unexpectedly passed: $invalid" >&2
    exit 1
  fi
done
