# Release download and verification

Only install a release whose GitHub tag, checksums, and artifacts agree.
Release packaging is reproducible and includes:

- flashable merged ESP32-S3 application image;
- Linux `pokeviewerctl`;
- offline content pack and provenance manifest;
- `SHA256SUMS`;
- release notes identifying the exact V2/non-touch target.

On a fresh Linux host:

```console
sha256sum --check SHA256SUMS
```

First verify the adjacent archive checksum. Then extract the archive and verify
its internal file checksums:

```console
sha256sum --check pokeviewer-v1.1.0.tar.gz.sha256
tar -xzf pokeviewer-v1.1.0.tar.gz
cd pokeviewer-v1.1.0
sha256sum --check SHA256SUMS
```

The checksum files must come from the GitHub release associated with the
verified `v1.1.0` tag. The publish workflow builds from a green `main` commit,
verifies the package, compares every draft asset with the local build, and
checks the public archive and tag after publication. Do not install an artifact
copied from an issue, chat, third-party mirror, or failed workflow.

Pokémon media is excluded from the source MIT licence; read
[Third-party notices](../THIRD_PARTY_NOTICES.md).
