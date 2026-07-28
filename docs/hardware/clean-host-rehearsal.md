# Exact-artifact clean-host rehearsal

This is the release-blocking procedure for issue #27. Do not start until the
seven-day qualification is complete and the manual `Release candidate`
workflow has produced a retained artifact from the intended commit.

Use a fresh supported x86-64 Linux installation with no Pokeviewer checkout,
workspace build output, maintainer shell history, or previously installed
Pokeviewer binary. The operator may use public release documentation and the
downloaded candidate only.

## Prepare private evidence

Copy `docs/evidence/rehearsal-template` to
`evidence/private/rehearsal-COMMIT`. Keep device paths, USB serials, home
directories, faces, names, reflections, location clues, and household details
out of every transcript and photograph.

Record only the exact hardware revision, candidate commit, SHA-256 values,
PASS/FAIL outcomes, and sanitized observations. A second adult must review the
five photographs before anything is made public.

## Download and verify

1. Download the retained `pokeviewer-v1.0.0-candidate-COMMIT` artifact from the
   successful manual workflow run.
2. Verify and extract it exactly as described in
   [release verification](../release-verification.md).
3. Confirm `BUILD-METADATA.txt` names the intended commit, exact V2/non-touch
   board, protocol 1, content format 1, schedule 1, and flash offset `0x0`.
4. Confirm the bundled CLI reports `pokeviewerctl 1.0.0`.
5. Record the archive, firmware, CLI, and content-pack hashes without recording
   the local download path.

Any mismatch ends the rehearsal. Do not repair or rename candidate contents.

## Erase, flash, and provision

With the battery disconnected and the board connected over USB:

1. Follow only `FLASHING.md` from the candidate.
2. Erase flash.
3. Flash the single merged image at `0x0`.
4. Photograph the adult setup screen after boot.
5. Run the bundled CLI handshake, set local RTC time, and require matching
   read-back.
6. Photograph the correct daily card and compare it with the deterministic
   schedule/frame hash for that local display date.
7. Confirm the device enters deep sleep.

Commands in the private transcript must replace the serial path with `DEVICE`
before saving.

## Scheduled transition

Leave the exact assembled device undisturbed through the next 07:00 local
alarm. Confirm one full refresh, the expected next card, and return to deep
sleep. Photograph the post-transition card and record the expected and observed
display dates and frame CRC.

## Recovery paths

Exercise these paths using only bundled documentation:

1. Remove all power long enough for RTC validity to be lost. Reconnect USB,
   photograph the setup screen, set the RTC with the bundled CLI, require
   read-back, and confirm normal rendering resumes.
2. With all power disconnected, disconnect the panel flex cable. Reconnect USB
   once and confirm the bounded `PANEL` policy without repeated resets.
   Disconnect power, restore and inspect the cable, then reset once and confirm
   recovery. Photograph only the safe, unpowered connection state and recovered
   card.

Stop immediately for heat, damage, swelling, odour, smoke, unstable wiring, or
unexpected repeated resets. Follow the [safety guide](../safety.md).

## Validate and review

Run from a separate trusted checkout used only to validate evidence:

```console
scripts/check-rehearsal-evidence.sh \
  evidence/private/rehearsal-COMMIT \
  DOWNLOAD/pokeviewer-v1.0.0.tar.gz
```

The script verifies the candidate, cross-checks its commit and payload hashes,
requires every release outcome to be `PASS`, checks the exact board identifier,
requires all five photographs to match their recorded SHA-256 values, and
rejects common private machine/device identifiers in the transcript. Keep the
photographs private; record only that reviewed images were provided when
updating the public issue.

The validator cannot judge photograph meaning, electrical safety, or whether an
observation is truthful. Two adults must complete the checklist and resolve
every defect. Any documentation or packaging change invalidates the candidate;
rebuild it and repeat this rehearsal from the beginning.
