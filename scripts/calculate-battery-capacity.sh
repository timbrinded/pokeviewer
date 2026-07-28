#!/usr/bin/env bash
set -euo pipefail

if (( $# != 6 )); then
  echo "usage: calculate-battery-capacity.sh SLEEP_MA ACTIVE_MA ACTIVE_SECONDS REFRESH_MA REFRESH_SECONDS USABLE_FRACTION" >&2
  exit 2
fi

awk -v sleep_ma="$1" -v active_ma="$2" -v active_s="$3" \
  -v refresh_ma="$4" -v refresh_s="$5" -v usable="$6" '
BEGIN {
  if (sleep_ma < 0 || active_ma < 0 || active_s < 0 ||
      refresh_ma < 0 || refresh_s < 0 || usable <= 0 || usable > 1) {
    print "invalid nonnegative measurement or usable fraction" > "/dev/stderr"
    exit 2
  }
  event_s = 3 * (active_s + refresh_s)
  if (event_s >= 72 * 3600) {
    print "active measurement leaves no sleep interval" > "/dev/stderr"
    exit 2
  }
  sleep_h = (72 * 3600 - event_s) / 3600
  active_mah = active_ma * 3 * active_s / 3600
  refresh_mah = refresh_ma * 3 * refresh_s / 3600
  sleep_mah = sleep_ma * sleep_h
  required = (active_mah + refresh_mah + sleep_mah) / usable
  printf "sleep_hours=%.6f\n", sleep_h
  printf "active_mAh=%.6f\n", active_mah
  printf "refresh_mAh=%.6f\n", refresh_mah
  printf "sleep_mAh=%.6f\n", sleep_mah
  printf "minimum_72h_mAh=%.3f\n", required
}'
