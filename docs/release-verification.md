# Release download and verification

Only install a release whose GitHub tag, checksums, and artifacts agree.
Release packaging is reproducible and includes:

- flashable merged ESP32-S3 application image;
- Linux `pokeviewerctl`;
- offline content pack and provenance manifest;
- `SHA256SUMS`;
- release notes identifying the exact V2/non-touch target and qualification
  status.

On a fresh Linux host:

```console
sha256sum --check SHA256SUMS
```

For a draft candidate, first verify the adjacent archive checksum, extract the
archive, then verify its internal file checksums:

```console
sha256sum --check pokeviewer-v1.1.0.tar.gz.sha256
tar -xzf pokeviewer-v1.1.0.tar.gz
cd pokeviewer-v1.1.0
sha256sum --check SHA256SUMS
```

For a final release, the checksum files must come from the GitHub release
associated with the verified `v1.1.0` tag. Compare the tag's commit with the
release notes and confirm the `Release matrix` check succeeded for that
commit. Do not install an artifact copied from an issue, chat, third-party
mirror, or failed workflow.

The release must remain a draft and must not be described as qualified while
the [device qualification](hardware/release-qualification.md) is pending. The
[isolated-environment rehearsal](hardware/clean-host-rehearsal.md) must
validate the final artifact, metadata, CLI, and documents before publication.
For v1.1.0, device behavior is qualified on source-equivalent installed
firmware. Exact candidate byte identity on the device is not claimed. Physical
images remain private. Public evidence states only that reviewed images were
provided.

Pokémon media is excluded from the source MIT licence; read
[Third-party notices](../THIRD_PARTY_NOTICES.md).
