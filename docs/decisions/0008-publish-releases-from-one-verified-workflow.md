---
status: proposed
date: 2026-07-30
decision-makers:
  - Project maintainer
---

# Publish releases from one verified workflow

## Context and Problem Statement

The v1.1.0 process split publication across a candidate workflow, retained
artifact, isolated-host rehearsal, device qualification, evidence validators,
approval environment, and final publish workflow. These manual stages repeated
automated package checks and made a release slow without detecting a distinct
release defect.

How should a maintainer publish a tested commit without losing package and
public-download integrity checks?

## Decision Drivers

- Require one clear maintainer action.
- Keep checks that can detect a corrupt, mismatched, or incorrectly published
  release.
- Reuse the successful CI result for the exact `main` commit.
- Remove release state that a maintainer must copy between workflows.
- Keep optional device checks separate from publication.

## Considered Options

- Keep the staged candidate and qualification process.
- Publish automatically after every version change on `main`.
- Publish from one manually dispatched, verified workflow.

## Decision Outcome

Chosen option: "Publish from one manually dispatched, verified workflow",
because the workflow dispatch is an explicit release action and all remaining
checks are automatic.

The workflow gets the version from the Cargo workspace. It requires a
successful `Release matrix` for its exact `main` commit. It then builds and
verifies one release, compares downloaded draft assets with that build,
publishes, and verifies the public archive, checksum, and tag.
Internal path dependencies do not repeat the application version. The
workspace packages are not published to a package registry.

The process does not require a candidate workflow, copied run ID, copied
commit, approval environment, isolated-host rehearsal, device qualification,
evidence template, photo review, or seven-day run.

### Consequences

- Good, because a release needs one maintainer action and no copied state.
- Good, because build, metadata, checksum, byte, and tag checks remain
  enforceable and repeatable.
- Good, because hardware experiments cannot delay an unrelated release.
- Bad, because the workflow does not require independent physical validation
  for each release.
- Bad, because a failed run must be inspected and started again after its cause
  is fixed.

### Confirmation

`actionlint` must accept the workflow. CI must pass without the removed
qualification validator. A local release package must pass
`scripts/verify-release.sh`.

## More Information

- [Publishing guide](../development/publishing.md)
- [Release verification](../release-verification.md)
- [Optional hardware validation](../hardware/validation.md)
