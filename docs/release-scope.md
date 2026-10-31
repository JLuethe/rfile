# Next experimental release: scope and PR decisions

Updated 2026-10-30. This is the current release-preparation contract. The
[release plan](11-github-release-plan.md) preserves the initial review and the
larger sequence. The existing public v0.1.0 remains unchanged.

## Scope

The next downloadable binary is the **default-feature `rfile` CLI for x86_64
Linux/glibc**, accompanied by complete source, notices, checksums, manual and
installation instructions. Default gzip/zlib inspection is included. A minimum
glibc version must be measured before labeling binary compatibility. Rust 1.87
remains the declared source-build minimum. No crates.io publication is planned.

| Surface | Current implementation | Release promise / remaining boundary |
|---|---|---|
| Default CLI and Rust engine | File-type detection, source/v21 magic, embedded rules, CDF/ELF, bounded matching and IO | Experimental supported profile; no full libmagic-replacement or security-audit claim |
| Source actions and diagnostics | Typed events, partial-load behavior and CLI `-c/-l/-C`; historical CLI/ABI matrices are recorded | DEBUG, broader warning/IO/discovery behavior and documented semantic differences remain |
| Extra codecs | Opt-in bzip2, XZ/LZMA/lzip, Zstd/LZ4, compress and first-entry ZIP | Available in source; omitted from the first default CLI binary; sustained qualification open |
| Linux sandbox | Opt-in Landlock/seccomp, auto/strict/off and actual enforcement reporting | Available in source; omitted from default binary; strict needs Landlock ABI >=5 and seccomp |
| C interface | 19 `magic_*` functions and a distinct `librfile_magic` library | Experimental source build; separate binary packaging and qualification deferred |
| Other platforms / ABIs | No new executed platform evidence | No new support claim |
| Security qualification | 25 fuzz targets and bounded queue infrastructure | Long campaigns, coverage/resource review and independent security audit remain open |

Do not confuse the release-preparation baseline with a final release candidate.
The baseline is `7a3ea756e79838e4c3ca252d23c3e9ff7ab8fe76`; this increment is
on `fix/release-readiness-baseline`. The proposed version is now `0.2.0-rc.1`. The final candidate commit and
artifact hashes will be frozen after fixes, independent retesting and packaging.
A report must identify the exact source snapshot it tested, including local
changes when applicable. Existing historical reports are not transferred to a
new candidate by changing their labels.

## PR disposition

GitHub was rechecked on 2026-10-30. Deferral below is an explicit release-scope
decision; it does not assert that a dependency is vulnerability-free.

| PR | Decision | Reason / next gate |
|---|---|---|
| #19 bounded fuzz queue | Keep as the development baseline; merge remains pending | Linux/MSRV checks pass. Claude's baseline covers Rust but not queue locking/interruption/accounting. Independent queue review/self-test is required before using it for qualification. It is not part of the shipped CLI runtime. |
| #20 Rust toolchain 1.120 | Closed during this increment | It changes the MSRV test away from declared 1.87 and its MSRV check fails. Dependabot configuration now leaves this deliberate minimum alone. |
| #21 rustix 1.1.6 | Defer from this default-CLI increment | Optional sandbox dependency; requires matching workspace/fuzz lockfiles, notices and independent kernel regression checks. Its PR currently changes only the workspace lock and platform manifest. |
| #22 lzma-rust2 0.24 | Defer from this default-CLI increment | Optional codec dependency; changes decoder accounting/padding behavior. Requires both lockfiles, notices and codec differential/boundary retesting. |
| #1 checkout v7 | Superseded in this branch; closure awaits CI | Both existing jobs now use the reviewed v7 commit, and candidate packaging uses pinned checkout/Python/upload actions. Hosted runner compatibility is documented upstream; fresh CI is required. |

PRs #1, #19, #21 and #22 remain open. Neither a green unit-test run nor this
scope decision closes their independent review gates. Revisit dependency
decisions if an advisory affects the shipped profile before artifact freeze.

## Tester/fixer workflow

Claude designs and executes tests, records reproducible findings, and retests
Codex fixes. Codex implements fixes and performs other validation: source review,
formatting, documentation consistency, Git/CI inspection and evidence/provenance
checks. Keep agent reports verbatim with separate corrections when necessary.
Use isolated checkouts and serialize feature runs. Production failures,
infrastructure failures and explicit skips have different dispositions.

The first independent follow-up reproduced zero fixture truncations despite
103 supplied oracle fixtures. The fix must report 103 files / 1,339 truncations,
fail for missing/empty required fixtures, and preserve the deterministic seed.
The verification runner must replace stale success on startup, record failures
and interruptions, retain run-specific logs and distinguish sandbox skips.

## Verification controls

`tools/verify.py` gives every run a unique directory under
`reports/verification-runs/`. Its `report.json` and logs are retained even after
later runs. `reports/verification.json` or `verification-extended.json` is a
mutable latest-run summary, with `running`, `failed`, `interrupted` or `passed`
status. The previous summary is preserved in the new run directory. SIGKILL or
power loss may leave `running`; that never means passed. A checkout lock prevents
two verification runs from mixing fixed-path output reports.

Full oracle verification sets `RFILE_REQUIRE_FIXTURES=1` and forwards the clean
`RFILE_REFERENCE` to the mutation test. Direct ignored mutation runs also require
fixtures by default. `--no-oracle` explicitly permits missing default fixtures
and records that coverage as skipped. To request that behavior directly, set
`RFILE_REQUIRE_FIXTURES=0`; an explicit `RFILE_REFERENCE` must still be usable.

Sandbox tests emit structured coverage records. Normal development runs can
record unsupported/not-compiled skips; they do not certify enforcement. Run
`tools/verify.py --all-features --require-sandbox` on a capable Linux runner to
require both CLI and kernel checks in debug and release. Direct focused tests
can set `RFILE_REQUIRE_SANDBOX=1`. No sustained campaign is started by these
options. Source/rule/test changes during verification invalidate the run.

Raw per-run output is ignored by Git because it can be large. Review and archive
the exact run report/logs before using them as release evidence; a committed
summary alone does not preserve ignored logs. No artifact is published by the
verification runner.

## Packaging work in progress

`tools/package_cli.py` creates a default CLI archive and complete vendored source
archive from an immutable Git snapshot; it rejects dirty checkouts. The artifacts
include notices, checksums, source/rule/lock identities, compiler versions and the
actual required glibc symbol version. `tools/check_cli_package.py` exercises the
exact archives, a private install prefix, safe uninstall behavior and a fresh-cache
offline source rebuild. `packaging/INSTALL.md` documents installation.

The release-candidate workflow builds on Ubuntu 22.04 and retains artifacts without
publishing a release. Independent package QA and the pinned-oracle matrix are
pending. Claude reached its five-hour quota after the reporting increment passed;
its reset is 23:50 America/Los_Angeles on 2026-10-30. Queue/package acceptance must
wait for independent execution after that reset. Codex continues build, provenance
and CI validation in the meantime.
