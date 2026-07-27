# Waveshare ESP32-S3-ePaper-1.54-EN V2 board contract

- Status: accepted for implementation; physical evidence incomplete
- Hardware issue: [H02 / #3][issue-3]
- Vendor source revision: [`3f96beedd2e8`][vendor-commit]
- Last reviewed: 2026-07-27

Pokeviewer supports only the non-touch
`ESP32-S3-ePaper-1.54-EN` V2 board, Waveshare SKU 32299. V1 firmware and pin
assumptions are incompatible and must not be used.

## Identity

| Property | V2 contract |
| --- | --- |
| MCU/package | ESP32-S3-PICO-1-N8R8 |
| CPU | Dual-core Xtensa LX7, up to 240 MHz |
| Internal SRAM/ROM | 512 KB / 384 KB |
| Flash | 8 MB |
| PSRAM | 8 MB octal |
| Display | 1.54-inch, 200 × 200, black-and-white e-paper V2 |
| Touch | Not populated on SKU 32299 |
| RTC | PCF85063ATL, I²C address `0x51` |
| Environment sensor | SHTC3, I²C address `0x70` |
| Audio codec | ES8311, I²C address `0x18`; unused by Pokeviewer |
| USB | Native ESP32-S3 USB Serial/JTAG on GPIO19/GPIO20 |
| Battery/charger | 3.7 V lithium input and ETA6098 charge/power-path circuit |

Waveshare says boards with a V2 mark use the N8R8 package and optimized sleep
power. The operating-system probe sees Espressif USB vendor/product
`303a:1001`, which is consistent with the native USB Serial/JTAG interface.
The device identifier is intentionally not recorded.

## GPIO allocation

Pins are frozen even when Pokeviewer does not initialize the attached
peripheral. This prevents a future change from accidentally powering hardware
or making the touch and non-touch SKUs behave differently.

| GPIO | Signal | Direction/active level | Pokeviewer use |
| ---: | --- | --- | --- |
| 0 | BOOT button | input, active low | adult recovery wake only |
| 3 | onboard LED | output | off in release firmware |
| 4 | `BAT_ADC` | ADC1 channel 3; 2:1 divider | bounded diagnostic sample |
| 5 | `RTC_INT` | input, active low | deep-sleep wake |
| 6 | `EPD3V3_EN` | output, low enables | panel power |
| 7 | touch reset | touch SKU only | reserved, never driven |
| 8 | `EPD_BUSY` | input; high means busy | panel state |
| 9 | `EPD_RST` | output, active low | panel reset |
| 10 | `EPD_D/C` | output | panel command/data |
| 11 | `EPD_CS` | output, active low | panel chip select |
| 12 | `EPD_SCLK` | SPI2 clock | panel clock |
| 13 | `EPD_SDI` | SPI2 controller output | panel data |
| 14 | audio MCLK | output | unused |
| 15 | audio BCLK | output | unused |
| 16 | audio input | input | unused |
| 17 | `BAT_Control` | output, high keeps battery path on | system latch |
| 18 | PWR button | input, active low | adult power/recovery wake |
| 19 | USB D− | USB | provisioning and diagnostics |
| 20 | USB D+ | USB | provisioning and diagnostics |
| 21 | touch interrupt | touch SKU only | reserved, never used |
| 38 | audio word select | output | unused |
| 39 | SD clock | SDMMC | unused |
| 40 | SD D0 | SDMMC | unused |
| 41 | SD command | SDMMC | unused |
| 42 | audio power enable | output, low enables | held high/off |
| 45 | audio output | output | unused |
| 46 | speaker amplifier enable | output | held off |
| 47 | I²C SDA | bidirectional | RTC bus |
| 48 | I²C SCL | output | RTC bus |

The display has no MISO connection. Firmware uses SPI mode 0 and a 5,000-byte
one-bit framebuffer. The panel datasheet limits write-mode SCLK to 20 MHz;
Pokeviewer must not copy the vendor example's contradictory 40 MHz setting.

## Shared I²C bus

The schematic and V2 examples place these devices on GPIO47/GPIO48:

| Address | Device | Required in v1 |
| ---: | --- | --- |
| `0x18` | ES8311 audio codec | no; rail remains off |
| `0x51` | PCF85063ATL RTC | yes |
| `0x70` | SHTC3 temperature/humidity sensor | no |
| `0x38` | FT6336 touch controller | must be absent on SKU 32299 |

An I²C scan is supporting evidence only. Firmware binds directly to the
required RTC address and must not infer board identity from a scan.

## Power behavior

- USB VBUS and the battery feed the board power path.
- GPIO17 high holds the battery-controlled system path on; driving it low asks
  the board to power off.
- GPIO6 low powers the e-paper rail. It must be high before deep sleep.
- GPIO42 low powers the audio section. It remains high for all Pokeviewer
  operation.
- The PCF85063 interrupt on GPIO5 and the PWR/BOOT inputs are valid deep-sleep
  wake sources.
- The e-paper keeps its image after the panel rail and MCU are inactive.
- Battery voltage is the calibrated GPIO4 reading multiplied by two. It is
  diagnostic, not a precise state-of-charge measurement.

The ETA6098 charger and connector do not make an arbitrary lithium cell safe.
Battery choice, protection, charge current, connector orientation, enclosure,
and supervision remain adult integration responsibilities.

## Battery connector safety gate

No cell may be attached until all of these checks are recorded:

1. Disconnect USB and ensure no battery is present.
2. Identify the connector ground pin with a multimeter continuity check against
   a known board ground.
3. Photograph the board connector and intended cell plug in the same
   orientation, with no identifying background.
4. Confirm the cell plug's positive lead aligns with board `VBAT`, regardless
   of wire colour.
5. Confirm the cell is a protected, single-cell 3.7 V lithium battery suitable
   for the ETA6098 charge configuration.

Reversed MX1.25 harnesses exist. Connector fit and wire colour are not evidence
of correct polarity.

## Physical verification status

| Check | Status | Evidence |
| --- | --- | --- |
| V2 marking | owner-confirmed | sanitized photos pending |
| USB controller identity | verified | `303a:1001`, serial omitted |
| Chip/package identity | pending | serial permission blocks probe |
| Flash size | pending | serial permission blocks probe |
| PSRAM size/mode | pending | diagnostic firmware required |
| Non-touch I²C population | pending | I²C diagnostic firmware required |
| Battery connector polarity | pending | physical multimeter check required |

The current `/dev/ttyACM0` node is owned by the `uucp` group and the active user
is not a member. Do not weaken the device node to world-writable permissions.
Add the development user to the appropriate device group through the host's
normal administration process, reconnect or re-login, then run the
[sanitized probe procedure](probe-procedure.md).

## Sources

- [Waveshare board documentation][vendor-docs]
- [Waveshare schematic][schematic]
- [Waveshare V2 examples at the pinned revision][vendor-commit]
- [1.54-inch e-paper V2 datasheet][panel]
- [PCF85063A datasheet][rtc]
- [ESP32-S3 datasheet][esp32s3]

[esp32s3]: https://documentation.espressif.com/esp32-s3_datasheet_en.pdf
[issue-3]: https://github.com/timbrinded/pokeviewer/issues/3
[panel]: https://files.waveshare.com/wiki/common/1.54inch_e-paper_V2_Datasheet.pdf
[rtc]: https://files.waveshare.com/wiki/common/Pcf85063atl1118-NdPQpTGE-loeW7GbZ7.pdf
[schematic]: https://files.waveshare.com/wiki/ESP32-S3-ePaper-1.54/ESP32-S3-Touch-ePaper-1.54-Schematic.pdf
[vendor-commit]: https://github.com/waveshareteam/ESP32-S3-ePaper-1.54/tree/3f96beedd2e8daa35996abd0c055a7d394336dfb
[vendor-docs]: https://docs.waveshare.com/ESP32-S3-ePaper-1.54
