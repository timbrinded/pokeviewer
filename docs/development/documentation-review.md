# Documentation release review

| Requirement | Document | Automated check | Current status |
| --- | --- | --- | --- |
| exact V2, non-touch board | `README.md`, user guide | link/docs CI | complete |
| Linux flash and RTC provisioning | user guide | CLI/parser and target-build tests | complete |
| offline 07:00 operation | user guide, product contract | schedule/state tests | complete |
| battery/small-parts/heat supervision | safety guide | link/docs CI | complete |
| all recovery codes | troubleshooting guide | failure-policy tests | complete |
| release download and verification | user and release-verification guides | candidate bundle verification | complete |
| exact bundled commands and filenames | user guide and `FLASHING.md` | candidate contents and CLI version checks | complete |
| PWR parent session and storage | user guide and state machine | host and device qualification | device evidence pending |
| battery estimate and limits | battery guide and ADR 0007 | curve/filter tests and goldens | device evidence pending |
| fresh Linux walkthrough | clean-host rehearsal | evidence validator | candidate and device evidence pending |
| sanitized screenshots and physical logs | qualification evidence | private review | blocked |
| licence and media exclusions | licence and third-party notices | repository review | complete |
| final public release audit | publishing guide and workflow | exact candidate enforcement | blocked |

Documentation warnings, Markdown structure, and live links are checked in CI.
The candidate verifier enforces the documented archive, firmware, and CLI
filenames and runs the bundled CLI version command. Literal device commands,
screenshots, and the complete fresh-host path remain release-blocking work for
the exact-artifact rehearsal because they require the final candidate and
physical V2 board. Physical images remain private.
