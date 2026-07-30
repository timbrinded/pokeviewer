# Seven-day physical qualification run

- Status: blocked before day 1 by physical qualification prerequisites
- Delivery issue: [Q23 / #24][issue-24]
- Last reviewed: 2026-07-29

The seven-day run cannot be replaced by simulation. This document fixes its
inputs and evidence so it can begin after the connected V2 board passes the
PWR, storage, battery-display, failure-injection, and release-matrix gates.

## Freeze the candidate

1. Record the full firmware and CLI commits and hashes in a copy of the
   [qualification template][template].
2. Confirm `Release matrix` is green for that exact commit.
3. Generate the seven expected cards from the chosen local start date:

   ```console
   cargo xtask qualification-schedule YYYY-MM-DD PATH/seven-day.csv
   ```

   The default schedule uses `50%` for each day. If the observed screen uses
   other 10 percent buckets, regenerate the file with all seven values:

   ```console
   cargo xtask qualification-schedule YYYY-MM-DD PATH/seven-day.csv \
     50,50,40,40,40,30,30
   ```

4. Flash once with the battery disconnected. Connect the protected battery,
   provision local time once, and disconnect USB. No network or manual refresh
   is permitted after the run starts.

## Daily capture

For days 1 through 7, capture one tightly cropped, metadata-stripped image
before 07:00 and one after the transition. Use the exact filenames in
`photos.csv`, record each SHA-256, and compare the weekday, name, types, sprite,
and framebuffer CRC-32 with `seven-day.csv`.

The sanitized run log records only:

```text
day=<1..7>
rtc_local=<YYYY-MM-DDTHH:MM:SS>
wake=Ext1
ext1_gpio5=true
alarm_pending=true
refresh=one
framebuffer_crc32=<8 lowercase hex>
next_wake=<YYYY-MM-DDT07:00:00>
reset=none
status=PASS
```

Any missed or duplicate transition, wrong or damaged frame, invalid RTC,
reset, manual intervention, wireless activity, or terminal error makes the
whole run `FAIL`. Preserve the failed sanitized evidence, identify the
corrective commit or hardware action, then restart at day 1 with a new run
identifier.

## Completion

After day 7, complete every checklist row, obtain second-adult review, and run:

```console
scripts/check-qualification-evidence.sh PATH
```

Only a passing validator output may unblock release. Serial access,
provisioning, RTC reads, rendering, panel-controller sleep, and panel rail-off
have passed. Timer-only deep sleep, RTC alarm assertion, one alarm-driven
`Ext0` wake in v1.0.0, production sleep entry, and private retained-image
evidence have also passed. V1.1.0 EXT1/PWR qualification, failure injections,
repeated short runs, and a green candidate release matrix remain
prerequisites, so no day-1 evidence or pass status exists.

Manual current measurement, discharge testing, and capacity certification are
outside v1.1.0. The public issue must state only that reviewed private images
were provided. Do not attach the images.

[issue-24]: https://github.com/timbrinded/pokeviewer/issues/24
[template]: ../evidence/qualification-template/
