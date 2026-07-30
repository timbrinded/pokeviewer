# V2 qualification result

- [ ] Board, panel, flash, PSRAM, RTC, USB, and non-touch identity pass.
- [ ] RTC set/read-back and oscillator-loss recovery pass.
- [ ] All 151 host render paths and committed goldens pass.
- [ ] Normal, recharge, and unavailable battery-status goldens pass.
- [ ] Panel full refresh completes in at most 10 seconds.
- [ ] 07:00 produces exactly one refresh and one return to deep sleep.
- [ ] A PWR tap causes no display refresh.
- [ ] A three-second PWR hold plus a valid USB frame opens the parent session.
- [ ] Storage mode clears the RTC, drops GPIO17, and enables no ESP wake source.
- [ ] Reset and power-loss recovery converge to the correct card.
- [ ] `RTC`, `PACK`, `PANEL`, `ALARM`, and `WAKE` recovery evidence passes.
- [ ] Three repeated short runs and the full seven-day run pass.
- [ ] Evidence privacy review and teardown pass.
