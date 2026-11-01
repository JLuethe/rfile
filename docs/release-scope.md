# Next experimental release: scope and PR decisions

Updated after Claude pass-11 acceptance and security-review scoping. This is the
current release-preparation contract. The [release plan](11-github-release-plan.md)
preserves the initial review and the
larger sequence. The existing public v0.1.0 remains unchanged.

## Scope

The next downloadable binary is the **default-feature `rfile` CLI for x86_64
Linux/glibc**, accompanied by complete source, notices, checksums, manual and
installation instructions. Default gzip/zlib inspection is included. The binary requires GLIBC 2.34 symbols and is tested on Ubuntu 22.04
(glibc 2.35); other glibc 2.34 distributions are not certified. Rust 1.87
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

The frozen, independently accepted `0.2.0-rc.1` candidate is
`ca9465f5f93eca75c80352a18a5de108e20762a0`, tree
`92b34ed5c86ea5737a4eb1f00e3ce085fe59dca5`. Packaging run 41326583263 produced
artifact 13091831221; required linux/msrv checks passed in run 41326558761.
Claude independently accepted those exact archives in pass 11. See
[the CI identities and checksums](../reports/ci-ca9465f-handoff.md) and
[the pass-11 acceptance record](../reports/claude-pass11-codex-review.md).
Runtime/build/installer code is unchanged from independently tested 8325cef.
Later handoff/documentation commits do not retarget these accepted archives.

The older unpublished draft used source
`e64ce034bee6a6c0959872d9b8f632d692dbfaa3`, tree
`91506a94437571b6fc3257f1d2e860824e19b727`, matching the tested a781c017 tree.
Its [historical acceptance record](../reports/release-0.2.0-rc.1/final/README.md)
retains those original identities. That draft's assets have not been replaced
by accepting ca9465f. Archive acceptance is not publication or security approval.

## PR disposition

The following records the GitHub review on 2026-10-30; it is not a fresh PR-state
check. Deferral below is a release-scope decision, not an assertion that a
dependency is vulnerability-free.

| PR | Decision | Reason / next gate |
|---|---|---|
| #19 bounded fuzz queue | Merged as `5316cb8` | Required Linux/MSRV checks and independent GPT-6 Luna queue tests passed: locking, duplicate rejection, partial accounting and deadline cleanup. Evidence is in `reports/release-0.2.0-rc.1/queue-qa/`. This is experimental infrastructure, not sustained qualification. |
| #20 Rust toolchain 1.120 | Closed during this increment | It changes the MSRV test away from declared 1.87 and its MSRV check fails. Dependabot configuration now leaves this deliberate minimum alone. |
| #21 rustix 1.1.6 | Defer from this default-CLI increment | Optional sandbox dependency; requires matching workspace/fuzz lockfiles, notices and independent kernel regression checks. At that review, its PR changed only the workspace lock and platform manifest. |
| #22 lzma-rust2 0.24 | Defer from this default-CLI increment | Optional codec dependency; changes decoder accounting/padding behavior. Requires both lockfiles, notices and codec differential/boundary retesting. |
| #1 checkout v7 | Superseded; closed after fresh CI passed | Both existing jobs and candidate packaging use pinned v7. Linux, MSRV and candidate artifact checks passed on `1883cdd`; the standalone dependency PR is redundant. |

PRs #21 and #22 were open and explicitly deferred in that record. Neither a green
unit-test run nor that scope decision closes their independent review gates.
Revisit dependency decisions if an advisory affects the shipped profile before
artifact freeze.

## Tester/fixer workflow

Claude performed the current independent pass series, including passes 6–11:
fix retests, both local source-verifier profiles, full golden truncations,
string16/container checks and exact CI-archive acceptance. Codex implements
production fixes and documentation and performs other validation: source review,
formatting, documentation consistency, Git/CI inspection and evidence/provenance
checks. Earlier release preparation also used GPT-6 Luna subagents when Claude's
quota was exhausted. Those historical records retain their actual agent and
candidate identities; they must not be presented as Luna testing of Claude's
later pass series. Keep agent reports verbatim with separate corrections when necessary.
Use isolated checkouts and serialize feature runs. Production failures,
infrastructure failures and explicit skips have different dispositions.

During earlier release preparation, a verifier run recorded zero fixture
truncations despite 103 supplied oracle fixtures. The subsequent fix requires
reporting 103 files / 1,339 truncations, failing for missing/empty required
fixtures, and preserving the deterministic seed.
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

## Packaging and final acceptance

`tools/package_cli.py` creates a default CLI archive and complete vendored source
archive from an immutable Git snapshot; it rejects dirty checkouts. The artifacts
include notices, checksums, source/rule/lock identities, compiler versions and the
actual required glibc symbol version. `tools/check_cli_package.py` exercises the
exact archives, a private install prefix, safe uninstall behavior and a fresh-cache
offline source rebuild. `packaging/INSTALL.md` documents installation.

The release-candidate workflow builds on Ubuntu 22.04 and retains artifacts without
publishing a release. For ca9465f, CI package/offline-rebuild checks passed 14/14
and minimal-runtime checks passed 13/13. Claude's pass-11 focused independent
acceptance verified provenance, all installed-file hashes/modes, archive contents,
INSTALL.md's I-1/I-2 wording, behavior, 19 installer assertions and a fresh-cache
offline rebuild. The executable and installer are byte-identical to pass 10's
accepted 3f9af8d archives; pass 10 ran the full 14,088-row golden scope on that
executable. Both pass-8 local verifier profiles passed on 8325cef; their actual
executed source identities are preserved rather than relabeled as new CI runs.

Historical Luna testing of earlier candidates, including FILETIME/ZIP64 fixes and
sanitizer/Miri work, remains attributed to its own reports. It does not establish
an independent security review of ca9465f. Security-review scoping has produced
a brief only; execution and sustained qualification remain open.

Both `linux` and `msrv` are required branch checks. CI includes debug/release
workspace tests for both profiles and a bounded oracle workflow; its all-feature
lane requires actual sandbox execution. The full oracle run also executes the
independent date-report regression tests against reports generated in that run.
Ordinary Python unit CI explicitly skips those two report checks without an
oracle-report directory; it never treats committed historical reports as fresh.

### Historical artifact work on earlier candidates

The earlier default pinned-oracle CI passed on 0c04563; its all-feature lane
exposed a gzip-helper ZIP64 text variant. Luna independently accepted the
FILETIME/ZIP64 corrections on the a781/e64 tree. Those results retain their
original candidate identities.

Luna's earlier artifact review found that a fresh Cargo cache hid rustup proxies when the
caller's PATH omitted `~/.cargo/bin`. The checker now preserves the selected
Cargo toolchain directory and defaults the fresh-cache build to one job (an
explicit `CARGO_BUILD_JOBS` override is honored). The PATH failure was independently retested successfully with one build job. Artifact QA also found that minimal images
without a system timezone failed before classification; the follow-up uses UTC
only for an absent implicit zone (and built-in TZ=UTC), preserving explicit
configuration errors. Luna accepted that earlier fix on `09f2c36` through 13
constrained-container checks, subsequently implemented in candidate CI. The older
e64 draft's workflow 41178070592 and its 14 package, 10 installer and 13
minimal-runtime checks remain historical evidence for those bytes. Current
accepted-candidate identities and Claude's pass-11 results are recorded above.
The draft has not been updated or published by these documentation corrections.

## Dependency-audit evidence status

`reports/dependency-audit.json` is explicitly labeled historical: its October 25
audit/provenance covers workspace lock SHA256
`1b8077fb97322634635d3f4963b9d9c0f52ee0bad3eec485644f90cbe69b0096`.
The frozen ca9465f Cargo.lock SHA256 is
`c7a1d2b995471286b3b89b3cfa5c18ece793d0a87c23c5e03ab0e6f2155f3f12`.
The original result and advisory revision are preserved; they are not relabeled
as a current-candidate audit. The verbatim report is retained as
`reports/dependency-audit-2026-10-25.raw.json`, matching its original provenance
checksum. A separate October 31 release audit record has matching frozen-lock
hashes in `reports/release-0.2.0-rc.1/audit-provenance.json`; that is also historical
evidence. A fresh audit for the security review against the frozen locks and a
recorded current advisory-database revision remains unrun and requires authorization.
