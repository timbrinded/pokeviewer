# Waveshare ESP32-S3-ePaper-1.54-EN V2 board contract

- Status: accepted for implementation; physical evidence incomplete
- Hardware issue: [H02 / #3][issue-3]
- Vendor source revision: [`3f96beedd2e8`][vendor-commit]
- Last reviewed: 2026-10-01

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
| Audio codec | ES8311, I²C address `0x18`; playback/capture unused; software-suspended |
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
| 0 | BOOT button | input, active low; 10 kΩ pull-up | ROM download mode at power-up; EXT1 restart wake |
| 3 | green LED (`LED_G`) | output, low enables; 24 kΩ to 3V3 | adult session and restart feedback |
| 4 | `BAT_ADC` | ADC1 channel 3; 2:1 divider | bounded battery-state sample |
| 5 | `RTC_INT` | input, active low | EXT1 wake; RTC-domain pull-up enabled |
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
| 42 | audio power enable | output, low enables | held low, including through deep sleep |
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
| `0x18` | ES8311 audio codec | rail remains powered; vendor software-suspend applied |
| `0x51` | PCF85063ATL RTC | yes |
| `0x70` | SHTC3 temperature/humidity sensor | sleep command only; unused otherwise |
| `0x38` | FT6336 touch controller | must be absent on SKU 32299 |

An I²C scan is supporting evidence only. Firmware binds directly to the
required RTC address and must not infer board identity from a scan.

## Power behavior

- USB VBUS and the battery feed the board power path.
- The board does not route USB VBUS to a dedicated ESP32-S3 sense input.
  USB Serial/JTAG traffic does not identify the power source. Firmware cannot
  distinguish USB power with no battery from a full battery reliably.
- GPIO17 high holds the battery-controlled system path on; driving it low asks
  the board to power off.
- GPIO6 low powers the e-paper rail. It must be high before deep sleep.
- GPIO42 low powers the audio section. Because the ES8311 shares SDA/SCL and
  clamps the bus when unpowered, it must remain low and be held low through deep
  sleep. Firmware applies the vendor ES8311 software-suspend sequence; the
  audio rail remains powered while the panel rail is off. The rail is not
  described as suspended; only the codec is software-suspended.
- The PCF85063 interrupt on GPIO5 is an open-drain, active-low EXT1 wake
  source. Before sleep, firmware must enable GPIO5's RTC-domain pull-up and
  disable its RTC-domain pull-down; configuring only the digital IO-mux pull-up
  is insufficient after the pin switches to RTC_IO.
- PWR (GPIO18) and BOOT (GPIO0) are adult EXT1 wake inputs. PWR also turns on
  the battery path in hardware through D9, T2, and Q4, so it starts a board
  whose GPIO17 latch is low.
- The SHTC3 is powered from the always-on 3V3 rail. It draws about 45 µA in
  its power-up idle state and 0.3 µA asleep, so firmware sends its sleep
  command at every boot.
- The speaker amplifier enable (GPIO46) has a 10 kΩ pull-down (R69), so the
  NS4150B stays in shutdown without firmware control.
- The orange LED is driven by the ETA6098 `STAT` output and shows charging.
  Firmware cannot control it.
- Hardware loads that firmware cannot remove: the 200 kΩ + 200 kΩ battery
  divider (about 9 µA), the MP1605 regulator quiescent current, and its
  feedback divider.
- The e-paper keeps its image after the panel rail and MCU are inactive.
- Battery voltage is the calibrated GPIO4 reading multiplied by two. Values
  from 2,500 mV through 4,500 mV are plausible. The retained value is bounded
  diagnostic data, not a precise state-of-charge or capacity measurement.
- A plausible value below 3,750 mV enters `Recharge`. A later plausible value
  at or above 3,850 mV clears it. An invalid scheduled observation preserves a
  complete retained `Recharge` snapshot; otherwise it commits `Unavailable`
  with `0` mV.
- Only a validated RTC alarm wake commits the retained scheduled battery
  snapshot. PWR and invalid-RTC paths do not sample or replace it.

The ETA6098 charger and connector do not make an arbitrary lithium cell safe.
Battery choice, protection, charge current, enclosure, and supervision remain
adult integration responsibilities.

## Physical verification status

| Check | Status | Evidence |
| --- | --- | --- |
| V2 marking | owner-confirmed | sanitized photos pending |
| USB controller identity | verified | `303a:1001`, serial omitted |
| Chip family/revision | verified | ESP32-S3 revision v0.2; device identifier omitted |
| Package identity | pending | physical marking or PSRAM probe required |
| Flash size | verified | 8 MB device probe |
| PSRAM size/mode | pending | diagnostic firmware required |
| RTC at `0x51` | verified | set/read-back and valid daily boot |
| Deep-sleep entry | verified | timer diagnostic slept once and woke by timer without a reset loop |
| GPIO5 RTC-domain pull-up | verified | alarm-driven EXT0 wake passed at a synthetic 07:00 boundary |
| Scheduled RTC wake/reboot | verified | retained verdict reported `Ext0` with the PCF alarm flag asserted |
| Battery millivolt accuracy | pending | one final retained USB value versus DMM comparison required |
| RTC-versus-PWR battery commit gate | verified | 2026-10-01: retained 3,902 mV through PWR and BOOT wakes; a synthetic 07:00 alarm wake committed 4,080 mV |
| BOOT restart and status LED | verified | 2026-10-01: tap ignored, one-second hold flashed and refreshed, 30-second hold slept with BOOT unarmed |
| Non-touch I²C population | pending | sanitized full-bus probe required |

Serial access is operational through the host's normal device group; device
permissions were not weakened. Complete the remaining
[sanitized probe procedure](probe-procedure.md) without publishing a device
path or identifier.

## Sources

- [Waveshare board documentation][vendor-docs]
- [Waveshare schematic][schematic]
- [Waveshare V2 examples at the pinned revision][vendor-commit]
- [1.54-inch e-paper V2 datasheet][panel]
- [PCF85063A datasheet][rtc]
- [ES8311 datasheet][es8311]
- [Pinned Waveshare ES8311 suspend sequence][es8311-suspend]
- [ESP32-S3 datasheet][esp32s3]

[es8311]: https://files.waveshare.com/wiki/common/ES8311.DS.pdf
[es8311-suspend]: https://github.com/waveshareteam/ESP32-S3-ePaper-1.54/blob/3f96beedd2e8daa35996abd0c055a7d394336dfb/02_Example/Arduino/08_Audio_Test/src/esp_codec_dev/device/es8311/es8311.c#L241-L258
[esp32s3]: https://documentation.espressif.com/esp32-s3_datasheet_en.pdf
[issue-3]: https://github.com/timbrinded/pokeviewer/issues/3
[panel]: https://files.waveshare.com/wiki/common/1.54inch_e-paper_V2_Datasheet.pdf
[rtc]: https://files.waveshare.com/wiki/common/Pcf85063atl1118-NdPQpTGE-loeW7GbZ7.pdf
[schematic]: https://files.waveshare.com/wiki/ESP32-S3-ePaper-1.54/ESP32-S3-Touch-ePaper-1.54-Schematic.pdf
[vendor-commit]: https://github.com/waveshareteam/ESP32-S3-ePaper-1.54/tree/3f96beedd2e8daa35996abd0c055a7d394336dfb
[vendor-docs]: https://docs.waveshare.com/ESP32-S3-ePaper-1.54
