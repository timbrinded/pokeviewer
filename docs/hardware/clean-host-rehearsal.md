# Artifact-only isolated-environment rehearsal

This is the release-blocking packaging rehearsal for v1.1.0. Device behavior
has a separate qualification procedure.

Use a fresh supported x86-64 Linux installation or a new ephemeral Linux
environment. The environment must have no Pokeviewer checkout, workspace build
output, maintainer shell history, or previously installed Pokeviewer binary.
Disable its network after the retained candidate is available. Give it
read-only access to only the candidate archive and adjacent checksum.

## Qualification boundary

The assembled qualification device keeps its battery connected. The v1.1.0
hardware evidence therefore comes from installed firmware with the same
firmware source and generated content as the release candidate. The exact
candidate bytes are not flashed to, or read back from, that device.

This rehearsal proves the candidate package, metadata, CLI, and documentation.
It does not prove on-device byte identity and does not replace the hardware
qualification.

## Prepare private evidence

Copy `docs/evidence/rehearsal-template` to
`evidence/private/rehearsal-COMMIT`. Keep device paths, home directories,
names, location clues, and other private environment details out of the
transcript.

Record only the candidate commit, SHA-256 values, PASS/FAIL outcomes, and
sanitized observations. No device photographs are required for this
artifact-only rehearsal.

## Verify the candidate

1. Download the retained `pokeviewer-v1.1.0-candidate-COMMIT` artifact from the
   successful manual workflow run.
2. Start the isolated environment.
3. Disable its network and confirm that no source checkout is available.
4. Give it read-only access to `pokeviewer-v1.1.0.tar.gz` and its adjacent
   `.sha256` file.
5. Follow the bundled `RELEASE-VERIFICATION.md`.
6. Confirm that the archive contains no absolute path or parent traversal.
7. Confirm that `BUILD-METADATA.txt` names the intended commit, exact
   V2/non-touch board, protocol 1, content format 1, schedule 1, and flash
   offset `0x0`.
8. Confirm that the bundled CLI reports `pokeviewerctl 1.1.0`.
9. Confirm that all required release documents are present and readable.
10. Record the archive, firmware, CLI, and content-pack hashes.

Any mismatch ends the rehearsal. Do not repair or rename candidate contents.
Do not flash or access a device during this rehearsal.

## Validate the evidence

Run from a separate trusted checkout used only to validate evidence:

```console
scripts/check-rehearsal-evidence.sh \
  evidence/private/rehearsal-COMMIT \
  DOWNLOAD/pokeviewer-v1.1.0.tar.gz
```

The script verifies the candidate, cross-checks its commit and payload hashes,
requires every artifact outcome to be `PASS`, and rejects common private
machine or device identifiers in the transcript.

Any documentation or packaging change invalidates the candidate. Build a new
candidate and repeat this rehearsal after such a change.
