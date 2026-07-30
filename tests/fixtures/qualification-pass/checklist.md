# Synthetic qualification fixture

- [x] Board, panel, flash, PSRAM, RTC, USB, and non-touch identity pass.
- [x] RTC set/read-back and oscillator-loss recovery pass.
- [x] All 151 host render paths and committed goldens pass.
- [x] Panel full refresh completes in at most 10 seconds.
- [x] 07:00 produces exactly one refresh and one return to deep sleep.
- [x] Reset and power-loss recovery converge to the correct card.
- [x] `RTC`, `PACK`, `PANEL`, `ALARM`, and `WAKE` recovery evidence passes.
- [x] PWR tap, PWR hold, and parent-session behavior pass.
- [x] Storage mode clears the RTC and disables wake sources.
- [x] Normal, low, and unavailable battery display states pass.
- [x] Three repeated short runs and the full seven-day run pass.
- [x] Evidence privacy review and teardown pass.
