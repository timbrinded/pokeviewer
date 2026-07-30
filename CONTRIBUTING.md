# Contributing

Pokeviewer is a small embedded project with a deliberately fixed v1 scope.
Start with the [product contract](docs/product-contract.md), the
[V2 board contract](docs/hardware/v2-board-contract.md), and the issue's native
blockers before changing code.

## Development commands

Cargo is the only build entry point:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny --locked check
cargo xtask help
```

Run the commands from the repository root with the pinned toolchain. Do not add
a Makefile, Justfile, shell build wrapper, or package-manager layer around them.
Repository automation belongs in `xtask`.

## Pull requests

- Work from a conventional branch such as `feat/daily-card` or
  `fix/rtc-alarm`.
- Keep commits reviewable and reference the owning issue.
- Update contracts and user documentation with behavior changes.
- Add host tests for pure logic and clearly identify tests requiring hardware.
- Do not weaken lint levels, generated-data checks, golden-image checks, or
  release-package checks to make a change pass.
- Keep credentials, child/household details, MAC addresses, USB serials, and
  private raw logs out of commits and CI artifacts.

The [publishing guide](docs/development/publishing.md) describes the one-step
release process.

Pokémon media is not covered by the MIT license. Read
[Third-party notices](THIRD_PARTY_NOTICES.md) before adding or updating assets.
