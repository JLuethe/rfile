# rfile restart checkpoint

Updated 2026-10-25. Read [the wrap-up plan](docs/09-wrap-up-plan.md), then the
relevant detailed specification. This is a checkpoint, not permission to publish
a release or a promise of active background work.

Read [execution and recovery](docs/10-execution-and-recovery-spec.md) for R6c
refinements and future supervisor requirements. Recovery automation is specified,
not installed. Security restrictions require review, not automatic resubmission.

## Current state

- Main is `5d8a8757324e9a499915954404d710ad645c5281`, R6c PR #14 merged;
  both hosted Linux and MSRV checks passed. R6c has 120 tests, 15,447 float and
  2,850 date comparisons, and 23 buildable fuzz targets; sustained gates are open.
- Active work is `feat/linux-confinement`, R7: optional Linux/glibc x86_64
  Landlock/seccomp adapter, ordered CLI planning, auto/strict/off/status, stable
  preopened strict inputs and private descriptor-relative compile publication.
- Focused CLI and eight kernel-child scenarios pass locally on Landlock ABI 10.
  The first full run passed 26 stages and 126 tests; 104 sandbox replay batches
  cover 5,760 operand comparisons across 411 fixtures. A separate 128-input test
  then found the BPF instruction ceiling. Contiguous approved FD ranges fix it;
  the 128-input/-p regression and descriptor-hole denial pass focused checks.
- The full rerun after the FD fix passed 26 stages. A subsequent reference audit
  found an older unrecorded seccomp patch in `upstream/src/seccomp.c`. Preserve
  that checkout. A separate clean pinned worktree is built at
  `.reference/file-clean`; the final clean replay in `r7-clean-verification.log`
  **passed all 26 stages**, with 126 debug/release tests and the same six named
  CLI differences. Reports record clean reference source/config/binary identities.
  Configuration/database hashes match the old build. Isolated default checks
  pass (124 tests), and MSRV 1.87 passes. No verification worker remains running
  at this checkpoint; query actual processes when resuming.
- No sandbox feature is enabled by default. Ordinary library calls never install
  confinement. Missing kernels are certification skips, not successful denial
  tests. Native tracing/other kernels/independent review remain qualification work.
- R8 C ABI is not implemented. Its source diagnostics/listing prerequisite is
  next after publishing R7. Native quantified-regex histories, broader semantics,
  sustained final-candidate fuzzing and independent review remain open.

## Resume

1. Inspect `git status --short --branch`, history and `git fetch origin`; preserve
   local edits. Query GitHub before repeating any push/merge after interruption.
2. Inspect `r7-clean-verification.log`, the worker/process tree and individual stage
   logs. A missing completion is UNKNOWN. Do not run default/all-feature tests
   concurrently using the same target directory: Cargo can replace the CLI
   executable while integration tests are running. That interference was observed
   once; it is recorded in `r7-initial-test-interference.log`.
3. R7 PR #15 has final local evidence; inspect its latest head and hosted checks.
   The initial implementation head passed Linux/MSRV. Wait for both checks on
   the final evidence/reference-audit commit before merging.
4. Publish/merge R7 PR #15 if still pending; query first to avoid duplicate actions.
   Keep initial feature delivery distinct from all SBX/security qualification.
5. Continue R8 from `docs/remaining/07-c-abi.md`: native source diagnostics/list
   behavior first, then all 19 ABI functions, client differential and packaging.
   Do not publish a replacement/stable release before its acceptance gates.

Cargo: `/home/john/.cargo/bin/cargo`. Prepare a clean test-only oracle with
`RFILE_REFERENCE=/absolute/clean/checkout bash tools/build_reference.sh`; use the
same `RFILE_REFERENCE` for `python3 tools/verify.py --all-features --benchmark`.
This machine currently uses `/home/john/Code/rfile/.reference/file-clean`. Pin:
`41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. Do not modify or discard the old patched checkout; the clean reference is now
separate. Reference tools reject tracked changes instead of checking HEAD alone.
The separately identified corrected-CDF oracle is auxiliary. Ignored binaries,
toolchains, corpora and `upstream/` are not guaranteed by a fresh clone.

## Bounded continuation request

“Read HANDOFF.md and docs/09-wrap-up-plan.md in my rfile repository. Reconcile
R7 source, live verification workers and GitHub state. Finish its next bounded
verification/publication step, then continue R8 as specified. Preserve named
compatibility decisions and update the checkpoint with actual results.”

This scopes authorized maintenance; it is not an access-control workaround.
For repeated product restrictions, preserve files, record the exact notice and
use the official reporting guidance linked in the plan.
