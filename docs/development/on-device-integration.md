# On-device daily-card integration

- Status: RTC alarm deep sleep integrated and physically qualified
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
powered through the active phase and deep sleep. Firmware applies the vendor
ES8311 software-suspend sequence before using the RTC, retains GPIO42 low with
its digital per-pin hold, and disables only the panel rail after refresh. The
rail is powered; only the codec is software-suspended. Audio capture and
playback are never configured.

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

On 2026-07-28, the connected V2 board running content revision 2 rendered
framebuffer CRC-32 `4f636e68`, read the RTC, put the panel controller to sleep,
and switched the panel rail off. An earlier low-power implementation reset
because it combined a pre-fix ESP-HAL sleep request with global digital-pad
autohold. The corrected implementation follows ESP-IDF sleep entry and uses
only GPIO42's per-pin hold. USB disappearance alone remains insufficient pass
evidence.

The release build now renders once, configures the fixed daily alarm, verifies
that refresh/configuration did not cross the planned boundary, and enters
GPIO5 active-low EXT0 deep sleep. The connected V2 board passed timer-only
sleep, awake PCF alarm/GPIO5 assertion, and one alarm-driven EXT0 wake with the
alarm flag asserted. After the real local RTC was restored and read back, the
production binary refreshed once, entered deep sleep, and remained absent from
USB throughout the bounded 45-second observation.

Sanitized panel photos and battery-side current measurements remain pending.

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
passed. Deep sleep is physically qualified. A hardware high-water measurement
plus sanitized panel photographs remain pending.

[issue-19]: https://github.com/timbrinded/pokeviewer/issues/19
