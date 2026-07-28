#!/usr/bin/env bash
set -euo pipefail

validator=scripts/check-qualification-evidence.sh
fixture=tests/fixtures/qualification-pass
mkdir -p target
work_dir=$(mktemp -d "$PWD/target/qualification-test.XXXXXX")
trap 'rm -rf -- "$work_dir"' EXIT

"$validator" "$fixture"

expect_failure() {
  local description=$1
  if "$validator" "$work_dir" >/dev/null 2>&1; then
    echo "invalid qualification evidence passed: $description" >&2
    exit 1
  fi
}

cp -R "$fixture"/. "$work_dir"/
sed -i 's/deep_sleep,0.100/deep_sleep,0.501/' "$work_dir/measurements.csv"
expect_failure 'deep-sleep current threshold'

cp "$fixture/measurements.csv" "$work_dir/measurements.csv"
sed -i '0,/\[x\]/s//[ ]/' "$work_dir/checklist.md"
expect_failure 'incomplete checklist'

cp "$fixture/checklist.md" "$work_dir/checklist.md"
printf '\nprivate_path=/home/example/device\n' >>"$work_dir/metadata.env"
expect_failure 'private host path'

cp "$fixture/metadata.env" "$work_dir/metadata.env"
sed -i 's/4f636e68/NOT_HEX_/' "$work_dir/seven-day.csv"
expect_failure 'invalid framebuffer hash'

cp "$fixture/seven-day.csv" "$work_dir/seven-day.csv"
sed -i 's/95014601/12345678/' "$work_dir/seven-day.csv"
expect_failure 'schedule mismatch'

cp "$fixture/seven-day.csv" "$work_dir/seven-day.csv"
sed -i 's/minimum_72h_mAh=9.791/minimum_72h_mAh=801/' "$work_dir/capacity.txt"
expect_failure 'insufficient usable battery capacity'
