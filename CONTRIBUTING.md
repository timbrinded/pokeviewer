# Contributing

Pokeviewer has a deliberately fixed v1 scope, set by the
[product contract](docs/product-contract.md). Read it before proposing a
feature. Behaviour outside it needs an accepted
[decision record](docs/decisions/README.md) before any code.

[AGENTS.md](AGENTS.md) is the short reference for commands, crate layout, and
the hard rules. It applies to human contributors as well as coding agents.

## Set up

- Host work needs only Rust; `rust-toolchain.toml` pins the version and
  `rustup` installs it on first use.
- Firmware work also needs the pinned Xtensa toolchain and `espflash`. Follow
  the [toolchain guide](docs/development/toolchain.md).

## Make a change

1. Branch from `main` with a conventional prefix, such as `feat/daily-card` or
   `fix/rtc-alarm`.
2. Keep each pull request to one concern and link its issue.
3. Update the contract, ADR, or user documentation in the same pull request
   when behaviour changes.
4. Before pushing, run:

   ```console
   cargo fmt --all --check
   cargo clippy --workspace --all-targets --locked -- -D warnings
   cargo test --workspace --locked
   ```

   Run `cargo deny --locked check` as well when you change dependencies. CI
   runs everything else; the [CI guide](docs/development/ci.md) lists each job
   and its local equivalent.
5. Write commit messages in Conventional Commits form, for example
   `fix(firmware): keep the panel rail off after refresh`.

## Testing

- Put logic in `pokeviewer-core` or an ungated firmware module and cover it
  with host tests. These run on every pull request; device-only behaviour does
  not.
- A visual change must update the goldens deliberately with
  `cargo xtask golden-update`. The pull request names the changed cards. See
  [rendering](docs/development/rendering.md).
- When a change alters hardware behaviour, run the one bounded device check
  listed in [hardware validation](docs/hardware/validation.md). Documentation,
  workflow, and host-tool-only changes need no device flash.

## Review and merge

- Merge only when the `Release matrix` CI check is green. `main` has no branch
  protection, so check it yourself. A red `main` also blocks releases.
- Reviewers check that the change stays inside the product contract, that the
  tests exercise the decision rather than the hardware adapter, and that
  nothing private or newly licensed slipped in.

## Privacy and licensing

The device belongs to a child. Never commit child or household details,
credentials, MAC addresses, USB serial numbers, private host paths, or raw
device logs. The [privacy and evidence rules](docs/privacy-and-evidence.md)
cover screenshots and logs.

Original code is MIT-licensed; Pokémon media is not. Read the
[third-party notices](THIRD_PARTY_NOTICES.md) before adding or changing assets.

## Release

A maintainer merges the version bump and release notes to `main`, then runs
the manually dispatched **Publish release** workflow. The
[publishing guide](docs/development/publishing.md) describes what it verifies.
