# Private clean-host rehearsal evidence template

Copy this directory to `evidence/private/rehearsal-COMMIT`. Do not edit or
commit this public template as if it were completed evidence.

Required private files:

- `rehearsal.env` with exact hashes and PASS/FAIL results;
- `terminal.txt` containing a sanitized command/result transcript;
- `checklist.md` signed by two adult reviewers;
- `setup.png`;
- `daily-card.png`;
- `scheduled-refresh.png`;
- `invalid-rtc.png`; and
- `failure-recovery.png`.

Replace every `REQUIRED` value in `rehearsal.env`. The transcript must use the
literal placeholder `DEVICE`, not the real serial path. Photographs must be
reviewed under `docs/privacy-and-evidence.md` and remain private. Record their
SHA-256 values in `rehearsal.env`; a GitHub issue may state that reviewed
images were provided, but must not attach them.
