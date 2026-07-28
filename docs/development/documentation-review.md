# Documentation release review

| Requirement | Document | Automated check | Current status |
| --- | --- | --- | --- |
| exact V2, non-touch board | `README.md`, user guide | link/docs CI | complete |
| Linux flash and RTC provisioning | user guide | CLI/parser and target-build tests | complete |
| offline 07:00 operation | user guide, product contract | schedule/state tests | complete |
| battery/small-parts/heat supervision | safety guide | link/docs CI | complete |
| all recovery codes | troubleshooting guide | failure-policy tests | complete |
| release download and verification | release-verification guide | candidate bundle verification | complete |
| fresh Linux walkthrough | clean-host rehearsal | evidence validator | blocked |
| sanitized screenshots and physical logs | qualification evidence | owned by #24 | blocked |
| licence and media exclusions | licence and third-party notices | repository review | complete |
| final public release audit | publishing guide and workflow | closed blocker enforcement | blocked |

Documentation warnings, Markdown structure, and live links are checked in CI.
Literal copy/paste validation of final release filenames and checksums belongs
to the draft-release and clean-host rehearsal because those artifacts do not
exist before packaging.
