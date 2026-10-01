# Publish a release

The `Publish release` workflow is the only publication path
([ADR 0008](../decisions/0008-publish-releases-from-one-verified-workflow.md)).

## Steps

1. On `main`, set the workspace version in `Cargo.toml`, update
   `release/RELEASE-NOTES.md` (its heading must be `# Pokeviewer vX.Y.Z`), and
   update every `pokeviewer-vX.Y.Z` and `pokeviewerctl-vX.Y.Z` name in the
   README quick start.
2. Wait for CI on that `main` commit to pass.
3. Open **Actions**, select **Publish release**, and select **Run workflow**.

The workflow takes no inputs. It stops if the ref is not `main`, if the commit
has no successful `Release matrix` check, or if the tag already exists.

## What the workflow checks

`scripts/build-release.sh` builds and checks the archive:

- the release notes heading matches the version, and every `vX.Y.Z` in the
  README names it;
- `cargo xtask content-build` leaves `content/generated` unchanged;
- the firmware has an entry point and fits its text, data, and pack budgets;
- neither shipped binary contains a home-directory path; and
- `scripts/verify-release.sh` accepts the archive: checksums, the exact file
  list, `pokeviewerctl --version`, the metadata version, and the pack hash and
  length recorded in the content manifest.

The workflow then creates a draft release, downloads every draft asset and
compares it byte for byte with the local build, publishes, downloads the
archive and checksum without credentials, compares those bytes again, and
checks that the tag points to the built commit. A failure before publication
deletes the draft and its tag. The workflow never deletes a published release.

The release binaries are built once. CI checks that the firmware's loaded ELF
sections and the content pack rebuild byte for byte, and the archive itself
is packed deterministically. Nothing compares the shipped merged image or CLI
with an independent second build, so do not describe a release as
reproducible.

No candidate run, approval environment, device check, photo review, or
evidence checklist is required.

## Release contents

`pokeviewer-vX.Y.Z.tar.gz` and its `.sha256` file, plus each file from the
archive as a separate asset:

- `pokeviewer-vX.Y.Z-esp32s3-v2.bin`, the merged image for offset `0x0`;
- `pokeviewerctl-vX.Y.Z-x86_64-unknown-linux-gnu`;
- `pokeviewer-v2.pack` and `content-manifest.json`;
- `BUILD-METADATA.txt`, `MANIFEST.txt`, and `SHA256SUMS`;
- `README.md` (with links rewritten to the shipped files or the tagged
  source), `RELEASE-NOTES.md`, `SAFETY.md`, and `TROUBLESHOOTING.md`; and
- `LICENSE` and `THIRD_PARTY_NOTICES.md`.

## Local package check

Run this when you change the package scripts. It needs the embedded toolchain,
`espflash`, `jq`, and a clean tree.

```console
version=$(cargo pkgid --locked -p pokeviewer-core)
version=${version##*#}
scripts/build-release.sh /tmp/pokeviewer-release
scripts/verify-release.sh \
  "/tmp/pokeviewer-release/pokeviewer-v$version.tar.gz"
```
