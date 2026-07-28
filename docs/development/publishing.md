# Publish v1.0.0

The `Publish v1.0.0` workflow is the only approved publication path. It is
intentionally unavailable until merged to `main` and must be protected by a
GitHub environment named `production-release` with a required adult maintainer
reviewer.
The workflow also reads the environment through GitHub's API and fails unless
a `required_reviewers` protection rule exists. GitHub documents the
[environment configuration and reviewer gate][github-environments].

Do not run it until the [publication checklist](../../release/PUBLISH-CHECKLIST.md)
is complete.

## Inputs

- `candidate_run_id`: the successful manual `Release candidate` run containing
  the exact rehearsed archive;
- `candidate_commit`: the full 40-character commit from that run and the
  rehearsal evidence; and
- `confirmation`: exactly `PUBLISH v1.0.0`.

The workflow refuses any ref other than `main`, any commit other than its own
checked-out SHA, an existing `v1.0.0` tag, an unsuccessful or mismatched
candidate run, or any open implementation, qualification, documentation,
candidate, or rehearsal issue in the v1 release chain (#9, #21, and #23–#27).

## Publication sequence

1. Download and verify the retained candidate archive.
2. Create a private draft release and attach only exact candidate payloads.
3. Download every draft asset and compare it byte-for-byte with the candidate.
4. Publish the final non-prerelease.
5. Download the public archive and checksum without release API credentials.
6. Verify the public checksum and tag commit.
7. Only then close release issue #28, root issue #1, and milestone #1.

The workflow does not compile, regenerate, rename, or substitute firmware,
content, or CLI payloads. If a step fails while the release is still a private
draft, the workflow removes that draft and its tag so a corrected, newly
qualified candidate can be retried. It never deletes a published release. A
failure after publication is a release incident and must not be hidden by
deleting evidence.

The final release notes state the exact V2/non-touch support boundary, fully
offline 07:00 behavior, RTC-loss limitation, battery-runtime caveat, safety
status, and unofficial project status.

[github-environments]: https://docs.github.com/actions/deployment/targeting-different-environments/using-environments-for-deployment
