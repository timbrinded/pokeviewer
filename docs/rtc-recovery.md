# Invalid-RTC setup and recovery

- Status: implemented
- Delivery issue: [T17 / #18][issue-18]
- Last reviewed: 2026-07-27

The firmware domain has only two RTC-gate outcomes:

- `Ready(DailySelection)` from one fresh, fully validated RTC reading; or
- `SetupRequired(SetupReason)` with no date or Pokémon selection attached.

Oscillator-stop/power-loss flags, transport/register failures, impossible
calendar fields, and years outside 2000–2099 all require setup. The gate never
stores a last-known date, invents a default, or converts an error into a
plausible card.

## Adult setup screen

The static [actual-pixel setup screen][screen] says:

```text
SET TIME
CONNECT USB
RTC RUN
POKEVIEWERCTL
```

It is rendered by the same bounded 5,000-byte framebuffer used for daily cards.
Its CRC-32 is `34e31d2e`; the committed one-bit PNG SHA-256 is
`7b38717e64138f684cae1119a64530d632de6ed97ffafadc2826c543d575f268`.
The e-paper retains this useful adult instruction while firmware waits or
sleeps; there is no child-facing menu and no network recovery path.

## Recovery transition

An RTC set command validates all seven local fields before mutation. Firmware
then reads the RTC back and returns that read-back in the protocol response.
The normal runtime must pass that newly read value through `assess_rtc`; only a
`Ready` result may select and render a Pokémon or configure the next 07:00
alarm. Provisioning at 06:59:59 correctly selects the prior display date.

Host tests cover oscillator loss, read failure, invalid leap dates, scheduling
range violations, valid boot bypass, and pre-07:00 recovery. Protocol tests
prove that invalid set requests leave the fake RTC unchanged.

Hardware photographs and the sanitized setup-to-card USB transcript remain
pending device access and full-card integration. They must not be inferred from
host screenshots.

[issue-18]: https://github.com/timbrinded/pokeviewer/issues/18
[screen]: evidence/setup-screen/README.md
