# Reproducible v1.0.0 candidate

The manual `Release candidate` workflow is the only approved candidate
assembler. It runs the complete reusable release-test matrix first, then makes
two clean builds from the workflow's immutable commit and requires their
normalized archives and SHA-256 files to match byte-for-byte.

## Run

1. Open **Actions**, then **Release candidate**.
2. Select the reviewed branch or exact commit containing the candidate.
3. Choose **Run workflow**.
4. Require `Required release tests` and
   `Reproducible v1.0.0 candidate` to pass.
5. Download the retained
   `pokeviewer-v1.0.0-candidate-COMMIT` workflow artifact.
6. Preserve the run URL, exact commit, and `reproducibility.txt` for rehearsal.

The workflow has read-only repository permission. It does not create a tag,
prerelease, release, package, deployment, or public binary.

## Candidate contents

`pokeviewer-v1.0.0.tar.gz` contains only:

- a merged V2 firmware image flashable at offset `0x0`;
- the Linux x86-64 `pokeviewerctl` binary;
- the compiled content pack and provenance manifest;
- build metadata, file manifest, and internal SHA-256 checksums;
- flash, user, safety, troubleshooting, and verification guides; and
- the source licence and third-party notices.

The adjacent `.sha256` verifies the archive itself. The verifier rejects unsafe
archive paths, missing or extra files, internal checksum failures, and a CLI
version other than `1.0.0`. It also cross-checks the board, targets, flash
offset, protocol version, content versions, pack length, and pack SHA-256
between build metadata, the generated content manifest, and the packaged
payload. Content revision values are derived from the manifest rather than
duplicated in the release script.

Raw PokéAPI cache, diagnostic firmware, test output, machine paths, serial
devices, secrets, and private logs are excluded by the explicit packaging
allowlist.

## Local maintainer check

With both pinned Rust toolchains and `espflash` 4.5.0 available:

```console
scripts/build-release-candidate.sh /tmp/pokeviewer-candidate
scripts/verify-release-candidate.sh \
  /tmp/pokeviewer-candidate/pokeviewer-v1.0.0.tar.gz
```

This local output is diagnostic only. Release evidence must come from the
retained manual workflow artifact.
