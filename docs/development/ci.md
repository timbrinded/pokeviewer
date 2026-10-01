# Continuous integration

The `CI` workflow (`.github/workflows/ci.yml`) runs four jobs on every pull
request and every push to `main`. A fifth job, `Release matrix`, passes only
when all four pass. The publish workflow requires it on the commit it
releases.

| Job | Checks | Defect it catches |
| --- | --- | --- |
| `Host checks` | `cargo fmt --check` | unformatted code |
| | `cargo clippy --all-targets -D warnings` | lint violations and type errors in every target |
| | `cargo test --workspace` | logic regressions, including the protocol codec, firmware handlers, and CLI |
| | `cargo doc` with `-D warnings` | broken intra-doc links and other rustdoc warnings |
| | `markdownlint-cli2` | malformed Markdown |
| | `lychee` over every tracked `.md` file | broken internal and external links |
| | `cargo deny check` | disallowed licenses, advisories, and sources |
| | `actionlint` | invalid workflow syntax and shell errors in workflows |
| `Offline content integrity` | `cargo xtask content-build`, then `git diff --exit-code -- content/generated` | committed pack, manifest, or contact sheet that does not match the cache and converter |
| `Visual and recovery goldens` | `cargo xtask golden-check`, `render-recovery-screens` compared with `docs/evidence/recovery-screens`, and `render-device-views` compared with `docs/images/device` | any changed pixel on a daily card or recovery screen, or a stale README device view |
| `ESP32-S3 release` | release firmware built twice and checked with `scripts/check-firmware-artifact.sh` | missing entry point, text without the content pack over 134,464 bytes, data over 16,384 bytes, pack over 262,144 bytes, or nondeterministic loaded sections |
| | the eight diagnostic images built once | target-only compile errors in diagnostic binaries |

On failure, the visual job uploads `visual-diff`. It holds, for each changed
case, `*-expected.png`, `*-actual.png`, `*-diff.png` (black where pixels
differ), and `*-report.txt` with the changed coordinates and hashes, plus the
rendered recovery screens and device views.

The firmware job uploads `esp32s3-release` with all nine ELF files, so a
reviewer can flash exactly what CI built. It also writes the size budgets to
the job summary. The section comparison fixes `SOURCE_DATE_EPOCH` to the
commit time and hashes `.rwtext`, `.data`, `.flash.appdesc`, `.rodata`, and
`.text`. Debug sections are excluded because they contain build paths and are
not flashed.

No CI job contacts PokéAPI. Third-party actions are pinned to commit SHAs, and
tool versions are pinned in the workflow.

## Run the checks locally

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo deny --locked check
actionlint
cargo xtask content-build && git diff --exit-code -- content/generated
cargo xtask golden-check target/visual-diff
```

The firmware checks need the [embedded toolchain](toolchain.md):

```sh
cargo xtask firmware-build
scripts/check-firmware-artifact.sh \
  target/xtensa-esp32s3-none-elf/release/pokeviewer-firmware \
  target/firmware-check
```
