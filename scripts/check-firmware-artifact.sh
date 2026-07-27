#!/usr/bin/env bash
set -euo pipefail

firmware=${1:?usage: check-firmware-artifact.sh FIRMWARE OUTPUT_DIR}
output_dir=${2:?usage: check-firmware-artifact.sh FIRMWARE OUTPUT_DIR}
text_max=${FIRMWARE_TEXT_MAX:-200000}
data_max=${FIRMWARE_DATA_MAX:-16384}
pack_max=${CONTENT_PACK_MAX:-65536}

mkdir -p "$output_dir/sections"
read -r text data _ _ _ < <(xtensa-esp-elf-size "$firmware" | tail -n 1)
pack_size=$(wc -c < content/generated/pokeviewer-v1.pack)
entry=$(readelf -h "$firmware" | awk '/Entry point address:/ { print $4 }')

if [[ "$entry" == "0x0" || -z "$entry" ]]; then
  echo "firmware has no executable entry point" >&2
  exit 1
fi
if (( text > text_max )); then
  echo "firmware text $text exceeds budget $text_max" >&2
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
sha256sum content/generated/pokeviewer-v1.pack > "$output_dir/content-pack.sha256"
cat > "$output_dir/budgets.txt" <<EOF
entry_point=$entry
text_bytes=$text
text_max=$text_max
data_bytes=$data
data_max=$data_max
content_pack_bytes=$pack_size
content_pack_max=$pack_max
EOF
cat "$output_dir/budgets.txt"
