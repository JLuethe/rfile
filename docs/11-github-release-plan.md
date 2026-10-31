# GitHub release readiness and independent QA plan

Review date: 2026-10-30 (America/Los_Angeles). This is a plan for **Rfile, the
Rust `file`/libmagic implementation**, not the `less` pager. Implementation
fixes belong to Codex; test execution and acceptance belong to a separate agent,
preferably Claude. This review does not authorize or perform a new publication.

Execution update: the user subsequently authorized starting steps 1 and 2,
with Claude testing/retesting and Codex fixing plus other validation. Current
scope and PR decisions are in [the release contract](release-scope.md).
The findings and first-pass reports below are retained as the dated review;
their original descriptions are not a claim that later fixes remain unfixed.

## Recommendation

Prepare the next **experimental x86_64 Linux/glibc GitHub release**, with a
downloadable default-feature CLI and source bundle. Keep extended codecs,
confinement and the C interface explicitly experimental and opt-in. Do not make
crates.io publication, additional operating systems or complete libmagic parity
prerequisites for this narrowly scoped release.

Maintain the existing, separate qualified-release endpoint: complete the
[hardening gates](remaining/06-hardening.md), independently review the unsafe
boundaries and close the supported compatibility contract before making stronger
security or replacement claims. Publishing another experimental release does
not close or waive those gates.

## Verified repository state

The following GitHub facts were queried live during this review, rather than
copied from the older handoff:

| Item | State |
|---|---|
| Repository | [JLuethe/rfile](https://github.com/JLuethe/rfile), already public |
| Existing release | [v0.1.0](https://github.com/JLuethe/rfile/releases/tag/v0.1.0), experimental prerelease, published 2026-10-24; no uploaded assets |
| Reviewed checkout | `feat/bounded-fuzz-queue`, `7a3ea756e79838e4c3ca252d23c3e9ff7ab8fe76`; initially clean and matching its local remote-tracking branch |
| Remote main | `5150a1ac832902cc1e866930ff2b2d45d3ef841a`, CLI database actions, PR #18 |
| Pending implementation | [PR #19](https://github.com/JLuethe/rfile/pull/19), bounded fuzz queue and provenance evidence; its head matches this checkout, Linux/MSRV checks passed, merge state clean |
| Main protection | PR required, `linux` required, applies to admins; zero required approving reviews; `msrv` runs but is not required |
| Dependency PRs | #1 checkout action, #20 Rust toolchain/MSRV action, #21 rustix, #22 lzma-rust2; disposition must precede candidate freeze |
| Package policy | Workspace packages are `0.1.0`, Rust 1.87, edition 2024, `publish = false`; GitHub distribution is independent of crates.io |

The engine, source and compiled magic support, CLI, Rust API, CDF/ELF enrichment,
bounded execution, optional codecs, locale/time support, optional sandbox and
experimental C API are already substantial implementations. The core forbids
unsafe Rust; the platform and FFI crates contain separate unsafe boundaries.
The pinned `file` 5.48 oracle is a test dependency, not a production wrapper.

Existing reports record a 33-stage full verification run, 145 ordinary
debug/release tests in that historical increment, extensive differential
comparisons and an isolated ABI package check. These are valuable **historical
evidence**, not a fresh independent pass of the next release candidate. Package
source provenance records commit `1198c0e`; older package reports and the public
tag refer to still other revisions.

## Findings from the Codex source and release review

These are release-process and static-review findings. They do not assert a
newly reproduced runtime defect.

| Priority | Finding and evidence | Required action |
|---|---|---|
| P0 | There is no single frozen release candidate: main, PR #19, existing tag and historical package evidence differ. | Select the release feature profile, finish PR disposition, record one candidate SHA and independently retest it. |
| P0 | Public status is contradictory. `docs/06-implementation-status.md` still says no kernel sandbox is implemented and broadly says `-c/-l` differs, despite newer implemented sandbox/action evidence. It lists 23 fuzz targets where the handoff records 25. The ABI README retains diagnostic gaps that were partly resolved. | Reconcile README, status, ABI README, handoff and release notes into one current support/gap table; preserve dated historical results. |
| P0 | No release asset workflow or CLI distribution package exists in `.github/workflows`; the existing release has zero uploaded assets. `tools/package_abi.py` packages the C library, not the CLI. | Produce installable CLI/source archives, checksums, notices, version/platform/features metadata, and test the exact archives on a clean machine. |
| P1 | `tools/package_abi.py` includes untracked allowlisted files and copies working-tree bytes, while its manifest's dirty flag ignores untracked files. It records state but does not refuse dirty source. | Release packaging must build from an immutable candidate checkout and verify included source against that commit. Keep any development packaging mode explicitly separate. |
| P1 | `tools/verify.py` writes its aggregate report only on success. A failed new run can leave an older passing aggregate at the same path. It lacks a top-level candidate SHA/run identity. | Give each run an immutable directory and record running/failed/interrupted/passed, candidate SHA, commands and artifact hashes; never treat an old report as the latest pass. |
| P1 | Hosted checks cover formatting, debug tests, Clippy, a default release build, database rebuilding and Rust C clients. They omit full pinned-oracle verification, release-mode tests, package installation, sustained fuzzing and the PR #19 queue self-test. | Add an explicit candidate verification workflow and bounded queue checks on a capable runner; retain separate hosted and isolated security lanes. |
| P1 | Claude identified test-coverage gaps, confirmed by source inspection: sandbox tests can return successfully after printing `SKIP`; the ignored mutation campaign silently omits fixture truncations when `upstream/tests` is absent and does not honor `RFILE_REFERENCE`. | Record skipped capability coverage separately; require actual enforcement for sandbox qualification. Make fixture location explicit and fail the qualification lane when required fixtures are missing. |
| P1 | FFI implementation-version JSON and pkg-config output hardcode `0.1.0` (`crates/rfile-ffi/src/lib.rs`, `tools/package_abi.py`). | Derive product versions from package metadata before the next tag; retain `magic_version() == 548` as the separate compatibility version. |
| P1 | `msrv` is not a required branch check; open PR #20 would move the tested version away from the declared 1.87 minimum. | Preserve an explicit Rust 1.87 lane unless deliberately changing the public minimum; review dependency/action updates rather than merging all green updates blindly. |
| Qualified-release blocker | Tracked all-target fuzz smoke reports only 81.238734 measured CPU seconds and `qualified: false`. No completed sustained campaign or independent security audit is recorded. | Complete the existing measured campaigns, coverage/resource review and independent audit for the qualified endpoint. A bounded Claude QA pass is not that audit. |

## Division of responsibilities

| Owner | Responsibility | Boundary |
|---|---|---|
| Codex | Implement production fixes, packaging, CI, documentation and agreed regression-test integration; explain each change and its intended behavior. | Cannot approve its own change as independently verified or edit expected results to conceal a failure. |
| Claude / independent testing agent | Design adversarial and compatibility cases, run the suites, minimize failures, author test-only reproducers, inspect artifacts, retest fixes and issue the candidate verdict. | Does not change production implementation. Test-only changes live separately until reviewed. |
| Maintainer | Resolve compatibility/product-scope choices, approve remaining nonblocking limitations, decide merges and publication. | Agent QA and passing CI do not themselves publish a release. |

Use separate worktrees or clean clones with separate build directories. Existing
test harnesses often hardcode `target/debug` or `target/release`; giving each
agent a whole checkout is safer than assuming `CARGO_TARGET_DIR` alone works.
Within a checkout, serialize default/all-feature runs to avoid replacing a CLI
while another test is executing it. Both agents must test the same recorded
candidate; after a fix, the tester owns the new acceptance result.

Each finding should contain: stable ID, severity, candidate SHA, features,
toolchain/platform, exact command, public synthetic reproducer, expected and
actual bytes/status, logs, proposed acceptance criterion, Codex fix SHA and
independent retest result. Use `OPEN -> REPRODUCED -> FIXED -> RETESTED -> CLOSED`;
only the testing agent closes a finding after reproducing the fix. Disputed
oracle behavior receives a specific compatibility decision, not a broad ignore.

## Ordered execution plan

### 1. Freeze scope and establish an independent baseline

Codex reconciles PR #19 and dependency PRs and updates release-facing status.
Recommended scope is the default CLI on x86_64 Linux/glibc; optional artifact
variants need their own evidence. Keep `publish = false`. Select a new version
and tag only after the candidate scope is fixed; do not move the old tag.

Claude starts from a clean candidate checkout, inventories prerequisites and
runs the baseline below. Record blocked and skipped checks separately from
failures. Deliver a prioritized findings ledger before broad implementation
changes. Review PR #19's locking, interruption and CPU accounting independently
before relying on its queue for sustained qualification.

Exit: one candidate identity, agreed shipped features, independent baseline,
and a bounded list of blockers with reproduction instructions.

### 2. Close release blockers through the tester/fixer loop

Codex fixes one coherent issue per commit or PR, including a regression where
appropriate. Prioritize crashes/panics, resource escapes, incorrect advertised
behavior, build failures and broken installs. Then address packaging/provenance,
version reporting and contradictory documentation. Do not expand into every
historical codec, locale or platform merely to claim completeness.

Claude reruns the original reproducer, adjacent boundary cases and the relevant
suite after each fix. Before candidate acceptance, rerun the complete release
matrix. Changed dependencies, rule bytes, feature flags and packaging scripts
also invalidate their affected evidence.

Exit: zero unresolved critical/high findings in shipped scope; remaining
compatibility differences are individually justified, tested and disclosed.

### 3. Make candidate verification repeatable

Codex adds CI lanes for the default/all-feature workspace, release-mode tests,
the actual declared MSRV, database identity and candidate artifact checks.
Add a manual or scheduled bounded oracle/security workflow with retained logs;
long campaigns need an isolated runner with the required sandbox facilities.
Pin the reference commit and record its clean source/configuration/binary hashes.
Make both Linux and MSRV required checks when updating repository protection.

Claude validates the workflow from a clean environment, verifies that a failing
check fails the job, and confirms missing prerequisites cannot produce a green
qualification report. A default `cargo test` alone excludes the FFI workspace
member; use `--workspace` for the release matrix.

Exit: reproducible commands and archived per-run evidence tied to the candidate.

### 4. Build and validate the actual release artifacts

Codex prepares a CLI archive containing `rfile`, the manual, install/uninstall
instructions, license/notice bundle and version/platform/feature metadata;
a complete corresponding source archive; and SHA-256 checksums. Record compiler,
glibc baseline, lockfile and embedded-rule hashes. If shipping the optional ABI,
also provide its distinct library/header/pkg-config/data bundle and retest its
configured install prefix. Source notices and dependency materials already in
the project must accompany the artifacts as applicable.

Claude downloads or copies the exact proposed archives into a clean environment
without the development checkout, cached binaries or upstream oracle. Verify
checksums, extraction, documented installation, help/version, default rule
discovery, file/stdin/MIME and error behavior, and uninstall. Check source bundle
completeness and locked offline rebuilding when vendoring is promised. Test the
oldest claimed glibc environment; a binary built on a newer host cannot establish
that compatibility. Compare two clean builds before claiming byte reproducibility.

Exit: artifact-level QA report, complete source/provenance, accurate notes and
an independent verdict on the exact hashes to upload.

### 5. Publish an experimental release, then pursue qualification

Prepare a reviewable GitHub release draft with candidate/tag identity, assets,
checksums, changelog, install instructions, supported platform/features and known
limitations. After maintainer approval, publish and have Claude smoke-test the
downloaded public assets. Any post-verification artifact rebuild requires new
checksums and retesting. Do not silently substitute binaries.

For the stronger qualified endpoint, execute the existing SEC/ABI gates rather
than relabeling smoke results: at least 24 measured CPU-hours per mandatory
enabled target, at least two seeds per target, required branch/resource evidence,
final-candidate replay, and independent unsafe/resource review. The current
all-feature 25-target set requires at least **600 CPU-hours**. Four fully utilized
workers imply a theoretical minimum of 150 hours (6.25 days), excluding setup,
triage and reruns; this is not a delivery estimate. A narrower release profile
needs an explicit justified target mapping, not an arbitrary smaller campaign.

## Independent test matrix

Run commands in the tester's isolated checkout; retain full logs and statuses.
Install missing tools/dependencies deliberately and record versions; do not
interpret infrastructure failures as detector failures.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test --workspace --release --all-features --locked
cargo +1.87.0 check --workspace --all-features --all-targets --locked
cargo build --release --locked
python3 tools/rebuild_database.py
cargo build -p rfile-ffi --all-features --locked
python3 tools/abi_clients.py --rust-only
python3 tools/unsafe_inventory.py

# In an isolated checkout, prepare a fresh clean pinned reference.
export RFILE_REFERENCE="$PWD/.reference/file-clean"
bash tools/build_reference.sh
python3 tools/verify.py --benchmark
python3 tools/verify.py --all-features --benchmark
```

Supplement with current dependency/advisory checks for workspace and fuzz locks,
independent black-box tests and synthetic boundary cases for truncated/oversized
rules, CDF/ELF offsets, compression expansion, regex exhaustion, cancellation,
non-UTF-8 names, symlinks, stdin/pipes and error channels. For optional features,
test individual codec feature combinations, actual sandbox availability/denial
and FFI ownership/state/descriptor preservation, including applicable sanitizer
and Miri checks. Track unsupported-host skips explicitly.

Until the mutation harness is corrected, the `RFILE_REFERENCE` commands above
do not redirect its hardcoded `upstream/tests` fixture path. Explicitly verify
its fixture count and arrange the clean pinned fixtures at that expected path
in the tester's checkout, or fix the harness before accepting that stage.
An offline check using a warm Cargo cache does not establish that a distributable
source bundle is self-contained; test the vendored archive independently.

Run queue self-tests only after building the required sanitizer targets and
checking that no campaign already owns the queue. Use disposable, resource-bounded
environments for untrusted parsing campaigns; preserve findings and manifests.

## This review's execution record

Codex reviewed source structure, manifests, CI, release tooling, existing reports,
status documents and live GitHub state. No implementation fix, merge, dependency
update, global install or publication was performed.

A bounded independent Claude review was launched against a clean `git archive`
of `7a3ea756e79838e4c3ca252d23c3e9ff7ab8fe76`, with source editing disabled in its
instructions and a process deadline. The first pass was static only because its
initial Git commands were not allowed by the harness. A second, test-only pass
completed with no permission denials. [Claude's verbatim test report](../reports/release-review-2026-10-30/claude-tests.md)
records these fresh results on Rust 1.99.1:

| Check | Independent result |
|---|---|
| Formatting | Passed |
| Workspace default tests, locked/offline | 143 passed, 0 failed, 1 ignored |
| Workspace all-feature tests, locked/offline | 145 passed, 0 failed, 1 ignored |
| Workspace all-feature/all-target Clippy, warnings as errors | Passed |
| Default release build, locked/offline | Passed |
| Release CLI help and README MIME classification | Passed |

The ignored test is the deterministic mutation campaign. Fresh MSRV execution,
release-mode tests, full oracle comparisons, package validation, dependency audit
and sustained security qualification were outside this bounded pass. Sandbox
test success alone does not prove that every capability branch executed; collect
explicit enforcement/skip evidence in the candidate lane. Offline dependency
resolution used this host's cache, not a new vendored-source installation.

The [evidence index](../reports/release-review-2026-10-30/README.md) preserves the
initial static report and explains corrections to its findings. The passing
baseline supports beginning release preparation; it is not final release sign-off.
