# Battery estimate and runtime scope

Pokeviewer v1.1.0 shows a coarse battery estimate. It does not make a battery
runtime claim.

The supported V2 board connects the cell monitor to GPIO4 through a 2:1
divider. Firmware uses ADC1 at 11 dB attenuation with ESP-HAL
`AdcCalCurve`. On each wake it:

1. waits 50 ms;
2. discards the first conversion;
3. takes 16 calibrated millivolt samples at 2 ms intervals;
4. sorts the samples and averages the two middle values;
5. applies the 2:1 divider; and
6. rejects values outside 2.5 V to 4.5 V.

The accepted value is mapped through this generic LiPo open-circuit-voltage
table:

| Estimate | Microvolts |
| ---: | ---: |
| 0% | 3,305,545 |
| 10% | 3,686,654 |
| 20% | 3,741,018 |
| 30% | 3,775,129 |
| 40% | 3,793,250 |
| 50% | 3,820,965 |
| 60% | 3,884,009 |
| 70% | 3,945,074 |
| 80% | 4,008,118 |
| 90% | 4,085,934 |
| 100% | 4,177,454 |

Firmware interpolates between table points and shows the nearest 10 percent.
It enters the low state below 15 percent and clears the state at or above 20
percent. While the low state is active, the display shows at most `10%` plus
the lightning icon and `CHARGE!`. An implausible sample shows `?%`.

This is a generic estimate. Load, temperature, cell age, charger state,
protection cutoff, and ADC tolerance can change the result. Firmware does not
use the estimate to stop charging, disconnect the cell, or enforce a safety
limit.

Manual current measurement, discharge testing, runtime certification, charger
certification, and a universal capacity claim are out of scope for v1.1.0.
