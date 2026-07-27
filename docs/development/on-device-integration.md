# On-device daily-card integration

- Status: host-complete; physical evidence pending
- Delivery issue: [I18 / #19][issue-19]
- Last reviewed: 2026-07-27

The release binary now composes the same repository-owned components used by
host evidence:

1. read the PCF85063A once and classify oscillator, transport, and calendar
   failures;
2. either select the passive display day or render the fixed RTC setup screen;
3. validate the committed Generation I pack in place from flash;
4. borrow the selected record without allocation;
5. render into the shared 5,000-byte panel-native framebuffer;
6. log its CRC-32 and pass those exact bytes to the V2 panel adapter; and
7. perform one bounded full refresh, put the panel to sleep, and disable its
   rail.

No Wi-Fi, BLE, SD, runtime API, heap-backed content loading, or child-facing
input is initialized. In setup mode, the existing bounded USB protocol remains
available. A successful RTC set and read-back causes a software restart into
the same normal boot path; reflashing is not required.

The target configuration links `linkall.x`. This is essential: without the
ESP-HAL linker script, a nominally successful ELF can retain an undefined
`_start` and omit the application, content pack, and hardware path.

## Integrated test evidence

Host tests exercise all 151 schedule positions through pack lookup and the
shared renderer. Bulbasaur, Charizard, Aerodactyl, Mr. Mime, and Pikachu cover
single/dual type, short/long name, punctuation, and sprite variation and
compare all 5,000 bytes against the reviewed visual goldens. The published epoch vector
`2026-01-01 07:00:00` resolves to cycle `0`, Pokédex `1`, Thursday.

Invalid RTC input compares against the setup-screen CRC-32 `063cff9d`. Each
successful device boot logs only the exact framebuffer CRC-32, not the local
date, device identifier, or host path.

## Static release budget

For commit-local builds, run:

```console
export RUSTUP_HOME="$PWD/.rustup-cache"
source .tools/export-esp.sh
cargo xtask firmware-build
rustup run esp-1.95.0.0 llvm-size \
  target/xtensa-esp32s3-none-elf/release/pokeviewer-firmware
rustup run esp-1.95.0.0 llvm-size -A \
  target/xtensa-esp32s3-none-elf/release/pokeviewer-firmware
```

The 2026-07-27 release build reports:

| Region or allocation | Bytes | Interpretation |
| --- | ---: | --- |
| linked text | 144,117 | code plus read-only payload reported by `llvm-size` |
| linked data | 7,156 | initialized RAM |
| explicit `.bss` | 40 | zero-initialized statics |
| linker `.stack` region | 328,392 | remaining reserved DRAM region, not measured use |
| application framebuffer | 5,000 | fixed stack allocation, included in stack use |
| offline pack source | 61,390 | flash-resident input before section/link packing |
| complete debug ELF | 2,818,948 | not the flashed payload size |

The linker leaves a 328,392-byte stack region after static placement, while the
largest application-owned working buffer is 5,000 bytes. This is substantial
static headroom for the fixed no-heap path. A hardware high-water measurement
and panel photographs remain pending serial permission; the linker region must
not be misreported as measured peak stack use.

[issue-19]: https://github.com/timbrinded/pokeviewer/issues/19
