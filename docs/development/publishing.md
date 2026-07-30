# Publish a release

The `Publish release` workflow is the publication path. It has no inputs and
does not use an approval environment.

## Publish

1. Merge the version and release notes to `main`.
2. Open **Actions**, select **Publish release**, and select **Run workflow**.

The workflow gets the version from the Cargo workspace. It stops if the
matching tag already exists or if the exact `main` commit does not have a
successful release matrix.

The workflow then:

1. builds the firmware, CLI, content, metadata, documents, and checksums;
2. verifies the release archive;
3. creates a private draft and compares every downloaded asset with the build;
4. publishes the release;
5. downloads the public archive without release API credentials; and
6. verifies the public bytes, checksum, and tag commit.

If a step fails before publication, the workflow removes its private draft and
tag. It never deletes a published release.

No separate candidate, device qualification, isolated-host rehearsal,
environment approval, or evidence checklist is required.

## Local package check

Run this only when you change the package scripts:

```console
version=$(cargo pkgid --locked -p pokeviewer-core)
version=${version##*#}
scripts/build-release.sh /tmp/pokeviewer-release
scripts/verify-release.sh \
  "/tmp/pokeviewer-release/pokeviewer-v$version.tar.gz"
```
