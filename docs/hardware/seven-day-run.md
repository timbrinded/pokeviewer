# Seven-day physical qualification run

- Status: blocked before day 1 by physical qualification prerequisites
- Delivery issue: [Q23 / #24][issue-24]
- Last reviewed: 2026-07-27

The seven-day run cannot be replaced by simulation. This document fixes its
inputs and evidence so it can begin after the connected V2 board passes the
one-wake, polarity, battery-current, and release-matrix gates.

## Freeze the candidate

1. Record the full firmware and CLI commits and hashes in a copy of the
   [qualification template][template].
2. Confirm `Release matrix` is green for that exact commit.
3. Generate the seven expected cards from the chosen local start date:

   ```console
   cargo xtask qualification-schedule YYYY-MM-DD PATH/seven-day.csv
   ```

4. Flash once, provision local time once, disconnect the host data connection,
   and use the documented battery or instrumented supply. No network or manual
   refresh is permitted after the run starts.

## Daily capture

For days 1 through 7, capture one tightly cropped, metadata-stripped image
before 07:00 and one after the transition. Use the exact filenames in
`photos.csv`, record each SHA-256, and compare the weekday, name, types, sprite,
and framebuffer CRC-32 with `seven-day.csv`.

The sanitized run log records only:

```text
day=<1..7>
rtc_local=<YYYY-MM-DDTHH:MM:SS>
wake=Ext0
alarm_pending=true
refresh=one
framebuffer_crc32=<8 lowercase hex>
next_wake=<YYYY-MM-DDT07:00:00>
reset=none
status=PASS
```

Any missed/duplicate transition, wrong or damaged frame, invalid RTC, reset,
manual intervention, wireless activity, terminal error, or power-threshold
failure makes the whole run `FAIL`. Preserve the failed sanitized evidence,
identify the corrective commit or hardware action, then restart at day 1 with
a new run identifier.

## Completion

After day 7, add the power and capacity results, complete every checklist row,
obtain second-adult review, and run:

```console
scripts/check-qualification-evidence.sh PATH
```

Only a passing validator output may unblock release. Serial access, provisioning,
rendering, and release-firmware deep-sleep entry have passed. The dedicated
one-alarm/one-wake diagnostic, polarity check, battery-side measurements, and
photographs remain prerequisites, so no day-1 evidence or pass status exists.

[issue-24]: https://github.com/timbrinded/pokeviewer/issues/24
[template]: ../evidence/qualification-template/
