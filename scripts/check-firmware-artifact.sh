#!/usr/bin/env bash
set -euo pipefail

firmware=${1:?usage: check-firmware-artifact.sh FIRMWARE OUTPUT_DIR}
output_dir=${2:?usage: check-firmware-artifact.sh FIRMWARE OUTPUT_DIR}
# The pack is embedded in .rodata, which `size` counts as text, so the code
# budget applies to text without the pack. 134,464 is the code room the
# earlier 200,000-byte text budget left beside a full 65,536-byte pack.
readonly code_text_max=134464
readonly data_max=16384
readonly pack_max=262144

mkdir -p "$output_dir/sections"
read -r text data _ _ _ < <(xtensa-esp-elf-size "$firmware" | tail -n 1)
pack_size=$(wc -c < content/generated/pokeviewer-v2.pack)
entry=$(readelf -h "$firmware" | awk '/Entry point address:/ { print $4 }')

if [[ "$entry" == "0x0" || -z "$entry" ]]; then
  echo "firmware has no executable entry point" >&2
  exit 1
fi
# A misread size would make the budget checks below pass vacuously.
if [[ ! "$text" =~ ^[1-9][0-9]*$ || ! "$data" =~ ^[0-9]+$ ]]; then
  echo "could not read firmware section sizes" >&2
  exit 1
fi
code_text=$(( text - pack_size ))
if (( code_text > code_text_max )); then
  echo "firmware text without the pack $code_text exceeds budget $code_text_max" >&2
  exit 1
fi
if (( data > data_max )); then
  echo "firmware data $data exceeds budget $data_max" >&2
  exit 1
fi
if (( pack_size > pack_max )); then
  echo "content pack $pack_size exceeds budget $pack_max" >&2
  exit 1
fi

for section in .rwtext .data .flash.appdesc .rodata .text; do
  xtensa-esp-elf-objcopy \
    --dump-section "$section=$output_dir/sections/${section#.}.bin" \
    "$firmware"
done
(cd "$output_dir/sections" && sha256sum *.bin) > "$output_dir/section-hashes.txt"
sha256sum content/generated/pokeviewer-v2.pack > "$output_dir/content-pack.sha256"
cat > "$output_dir/budgets.txt" <<EOF
entry_point=$entry
text_bytes=$text
code_text_bytes=$code_text
code_text_max=$code_text_max
data_bytes=$data
data_max=$data_max
content_pack_bytes=$pack_size
content_pack_max=$pack_max
EOF
cat "$output_dir/budgets.txt"
