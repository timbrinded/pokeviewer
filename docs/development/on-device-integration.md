# On-device daily-card integration

- Status: awake-first build and physical stability gates passed; deep sleep pending
- Delivery issue: [I18 / #19][issue-19]
- Last reviewed: 2026-07-28

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

The shared I²C bus requires GPIO42 to remain low, so the audio rail stays
powered while the staged runtime remains awake. Firmware applies the vendor
ES8311 software-suspend sequence before using the RTC, retains GPIO42 low as an
ordinary output, and disables only the panel rail after refresh. The rail is
powered; only the codec is software-suspended. Audio capture and playback are
never configured.

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

Invalid RTC input compares against the setup-screen CRC-32 `34e31d2e`. The
release success record contains the exact framebuffer CRC-32 and the next-wake
calendar date. Hardware diagnostics additionally log the RTC datetime. Neither
path intentionally logs a device identifier or host path, but captured output
must still be sanitized before publication.

The connected V2 board rendered framebuffer CRC-32 `d227338a`, read the RTC,
put the panel controller to sleep, and switched the panel rail off. The
attempted low-power path did not prove stable deep sleep: a measured 15-second
trace showed USB re-enumeration and another boot about 2.3 seconds later. USB
disappearance alone is therefore not pass evidence.

The current development build renders once, remains awake, and polls the RTC
every 30 seconds until the strictly future 07:00 boundary. It is a bring-up
baseline, not the v1 release behavior. On 2026-07-28, an ordinary boot remained
continuously present over USB for 607 seconds with zero state changes. A
synthetic near-07:00 run made one reset and refresh, planned the following
day's boundary, then remained continuously present for 608 seconds with zero
state changes.

Sanitized panel photos, battery-side current measurements, the battery
polarity gate, and isolated sleep/wake qualification remain pending.

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

Record both `llvm-size` outputs against the exact release-candidate commit;
linked sizes change when the hardware boundary changes and must not be copied
from an earlier build. The fixed application framebuffer is 5,000 bytes. A
linker-reported stack region is remaining address-space allocation, not a
measurement of peak stack use. USB flashing and a daily-card render have
passed. Deep sleep remains unqualified, and a hardware high-water measurement
plus sanitized panel photographs remain pending.

[issue-19]: https://github.com/timbrinded/pokeviewer/issues/19
