---
status: accepted
date: 2026-07-29
decision-makers:
  - Project maintainer
---

# Use a generic LiPo OCV battery estimate

## Context and Problem Statement

The daily card needs a discreet battery percentage and a clear low-battery
message. The supported board exposes the battery through a 200 kΩ by 200 kΩ
divider on GPIO4. The selected EEMB LP402535 specification identifies the
cell, but it does not publish a voltage-to-capacity discharge curve.

A cell-specific curve would require a controlled discharge and voltage
measurement. That work is not required for v1.1.0. The percentage must remain
useful without presenting a generic voltage estimate as a fuel gauge.

Should the firmware omit percentage, require cell characterization, or use a
documented generic LiPo open-circuit-voltage table?

## Decision Drivers

- Avoid false claims of cell-specific accuracy.
- Avoid manual discharge and multimeter calibration for v1.1.0.
- Sample before the high-current panel refresh.
- Keep estimation deterministic and allocation-free.
- Make low-battery status visible without controlling battery safety.
- Permit a later table replacement without changing the card API.

## Considered Options

- Show only a low-voltage warning.
- Block release on an LP402535 discharge characterization.
- Use a documented generic LiPo OCV table and coarse display steps.

## Decision Outcome

Chosen option: "Use a documented generic LiPo OCV table and coarse display
steps."

Firmware uses calibrated ADC1 curve sampling on GPIO4 at 11 dB attenuation,
applies the board's 2:1 divider, filters 16 readings, and linearly interpolates
the default Zephyr LiPo OCV table. The result is clamped to 0 through 100 and
shown in 10% steps.

The recharge state enters below 15% and clears at 20%. While it is active, the
display shows at most `10%` with a custom lightning glyph and `CHARGE!`.
Plausibility failures show `?%`.

The board does not expose a dedicated USB VBUS-sense input. ESP-IDF requires
such an input for reliable USB-power detection on a self-powered ESP32-S3.
USB Serial/JTAG traffic and the battery-divider voltage are not substitutes
for VBUS sensing. Firmware does not infer the power source from them. USB
power with no battery can therefore clamp the displayed estimate to `100%`.

The estimate is status information only. It does not control shutdown,
charging, or battery safety. Battery and charger certification are outside
this firmware decision, and the release makes no production-safety claim for
a selected cell.

### Consequences

- Good, because v1.1.0 gains useful full, middle, and low status without a
  manual characterization run.
- Good, because the pure interpolation and hysteresis rules are host-testable.
- Good, because coarse steps avoid displaying unsupported precision.
- Bad, because load, temperature, cell age, and chemistry variation can move
  the displayed estimate.
- Bad, because a generic table is not an LP402535 calibration.
- Bad, because a USB-powered observation can differ from a rested,
  battery-only observation.
- Bad, because firmware cannot identify USB power reliably.

### Confirmation

Host tests must cover every table point, interpolation, clamp, invalid sample,
rounding boundary, and recharge hysteresis transition. Reviewed framebuffer
goldens must cover normal, recharge, and unavailable status.

The V2-board qualification needs only a valid GPIO4 sample and visible status.
It does not require a manual discharge curve, DMM calibration, physical
low-battery test, or USB-power detection.

## Pros and Cons of the Options

### Show only a low-voltage warning

- Good, because it makes the fewest accuracy claims.
- Bad, because it does not meet the percentage requirement.

### Characterize the exact cell

- Good, because the estimate can better match one cell under one test load.
- Bad, because it requires equipment, time, and a repeatable discharge method.
- Bad, because one curve still changes with temperature and cell age.

### Use a generic LiPo table

- Good, because it provides a bounded approximation from published data.
- Good, because a later table can replace it behind the same renderer type.
- Bad, because the percentage remains approximate.

## More Information

- [Product contract](../product-contract.md)
- [Board and power contract](../hardware/v2-board-contract.md)
- [Zephyr default LiPo OCV table](https://docs.zephyrproject.org/latest/doxygen/html/group__devicetree-battery.html)
- [EEMB LP402535 specification](https://eemb.oss-accelerate.aliyuncs.com/uploads/20230315/7900a5b46fe77f926d269d99ff3a8dfa.pdf)
- [ESP-IDF self-powered USB requirements](https://docs.espressif.com/projects/esp-idf/en/v5.5/esp32s3/api-reference/peripherals/usb_device.html#self-powered-device)
