# rfile restart checkpoint

Updated 2026-10-25. Read [the wrap-up plan](docs/09-wrap-up-plan.md), then the
relevant detailed specification. This is a checkpoint, not permission to publish
a release or a promise of active background work.

Read [execution and recovery](docs/10-execution-and-recovery-spec.md) for R6c
refinements and future supervisor requirements. Recovery automation is specified,
not installed. Security restrictions require review, not automatic resubmission.

## Current state

- R7 PR #15 is merged on main at `c53bc2ac91d7e4a821231f12e0c4fd6bc65ad8ba`.
  Hosted Linux/MSRV checks passed. Final clean-reference full replay passed all
  26 stages with 126 debug/release tests, 124 default tests and 5,760 sandboxed
  operand comparisons. Kernel policy remains opt-in; qualification gaps remain.
- The old `upstream/` checkout has a preserved, previously unrecorded seccomp
  allowlist patch. Never treat it as clean or discard its edits. Use the separately
  built `.reference/file-clean` at the exact pin. Tools now reject tracked source
  modifications; reference-audit.json records the correction and final identities.
- Active branch is `feat/c-api-diagnostics`, R8 experimental ABI preview. All
  19 exports exist in an isolated crate, with panic/cookie/pointer ownership
  boundaries, native parameter widths and compiled-header diagnostics, bounded
  source snapshots, legacy list/check dumps and distinct library identity.
  It is **not drop-in certified**. Next is the typed parser diagnostic stream;
  six malformed-source cases remain frozen in data/abi-known-gaps.json.
- Full replay passed 31 stages, 134 ordinary debug/release tests, 132 isolated
  default tests, MSRV 1.87 and 582/582 successful source-check dump cases.
  ABI state comparison: 43/49 exact; six recorded diagnostic gaps. C/C++ clients
  (including C ASan/UBSan), 20-symbol/SONAME/linkage checks and pinned python-magic
  0.4.27 pass. ABI sanitizer smoke: 160,079 cases, 30.900628 measured CPU seconds.
  The core still forbids unsafe; reports/unsafe.json includes the separate FFI.
- Packaging tools now create an isolated tree with headers, pkg-config, rule
  sources/notices and a vendored source archive. An offline source-package check
  found an omitted compile-time test fixture; the packaging allowlist was fixed.
  The rebuilt 62d7b13 package passed dynamic/static clients, default discovery
  and an offline vendored source rebuild; reports/abi-package.json records it.
- Fuzz campaigns now require bubblewrap isolation. CPU is measured by an inner
  wait4 parent, because the outer namespace supervisor did not include descendant
  CPU on this host. No sustained campaign or independent review gate has passed.
  Do not resume old manifests across different supervisor/source/binary hashes.
- PR #16 is open for the preview; inspect latest hosted checks before merging.
  cargo-audit 0.22.2 completed against a recorded RustSec database revision;
  see reports/dependency-audit*.json for results and exact scope.
  No global install, system replacement or new release was performed.

## Resume

1. Inspect Git/workers/GitHub before repeating any action; preserve local edits.
   R7 is already merged. Rebase the R8 branch onto origin/main if not yet done.
2. Use `RFILE_REFERENCE=/home/john/Code/rfile/.reference/file-clean` for oracle
   tooling. `upstream/` is the preserved modified historical checkout.
3. Checkpoint the tested R8 preview in a PR; require Linux and MSRV checks.
   Rebuild/verify the isolated package including its offline source archive.
4. Continue the typed parser diagnostics prerequisite in docs/remaining/07-c-abi.md.
   Preserve strict Rust parsing; implement native diagnostic event ordering and
   warning state without quadratic prefix reparse or Display-string heuristics.
5. Run affected differential/ownership checks and then the full matrix. Keep
   default/all-feature CLI tests serialized or use isolated target directories.
6. Continue R9 audit/coverage and isolated sustained campaigns; retain actual
   independent review and residual fidelity/platform gates. Keep implementation
   delivery separate from security qualification; do not claim completed work
   based on symbol counts, successful compilation or short fuzz smoke.

Cargo: `/home/john/.cargo/bin/cargo`. Prepare a clean test-only oracle with
`RFILE_REFERENCE=/absolute/clean/checkout bash tools/build_reference.sh`; use the
same `RFILE_REFERENCE` for `python3 tools/verify.py --all-features --benchmark`.
This machine currently uses `/home/john/Code/rfile/.reference/file-clean`. Pin:
`41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. Do not modify or discard the old patched checkout; the clean reference is now
separate. Reference tools reject tracked changes instead of checking HEAD alone.
The separately identified corrected-CDF oracle is auxiliary. Ignored binaries,
toolchains, corpora and `upstream/` are not guaranteed by a fresh clone.

## Bounded continuation request

“Read HANDOFF.md and the remaining-work specs. Reconcile R8 source, workers and
GitHub state. Continue the source diagnostics/listing prerequisite, then the
specified C ABI. Preserve compatibility decisions and checkpoint actual results.”

This scopes authorized maintenance; it is not an access-control workaround.
For repeated product restrictions, preserve files, record the exact notice and
use the official reporting guidance linked in the plan.
