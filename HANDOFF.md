# rfile restart checkpoint

Updated 2026-10-24. Read [the wrap-up plan](docs/09-wrap-up-plan.md), then the
relevant detailed specification. This is a checkpoint, not permission to publish
a release or a promise of active background work.

Read [execution and recovery](docs/10-execution-and-recovery-spec.md) for R6c
refinements and future supervisor requirements. Recovery automation is specified,
not installed. Security restrictions require review, not automatic resubmission.

## Current state

- R6c is implemented on `feat/format-time-edges`; inspect Git/PR state for the
  latest checkpoint. Main through `d54ded9` was incorporated first.
- All 25 full verification stages passed: 120 ordinary debug/release tests,
  15,447 exact float and 2,850 exact date comparisons. Default/MSRV checks pass.
- Final formatter and timezone sanitizer smoke reports pass and match source/
  binary identities. There are 23 targets; no sustained gate is complete.
- Native quantified-history restrictions, custom TZif/host-state edge cases,
  sandbox, C ABI, independent review and other-platform certification remain.
- The next implementation is R7 Linux CLI confinement. Review ordered `-f`
  behavior, baseline-before-parse requirements and strict preopened-input design.
- Verification workers finished at this checkpoint. Recheck live process state;
  older log files do not establish whether a newly launched job completed.

## Resume

1. Inspect `git status --short --branch` and recent history; fetch origin.
   Preserve local changes. The local branch named main was behind at review;
   do not confuse it with the current active branch or origin/main.
2. Finish publishing R6c if its PR is still pending, then branch from current
   main for R7. Inspect `src/cli.rs`, `src/filesystem.rs`, the platform crate
   and `docs/remaining/05-sandbox.md`.
3. Preserve native CLI event ordering when planning execution. Prove actual
   denied operations in child processes and never restrict library callers.
   Keep full semantics gaps explicit; do not broaden allowed mismatch patterns.
4. Run focused tests/differential; once changes pass, run default/all-feature
   checks, MSRV and `python3 tools/verify.py --all-features --benchmark`.
5. Update evidence/docs/handoff, push a PR, and wait for Linux and MSRV CI before
   merging. Advance through the remaining plan with separate checkpoints.

Cargo: `/home/john/.cargo/bin/cargo`. Prepare the test-only oracle with
`bash tools/build_reference.sh`; pin:
`41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. Do not modify the primary oracle.
The separately identified corrected-CDF oracle is auxiliary. Ignored binaries,
toolchains, corpora and `upstream/` are not guaranteed by a fresh clone.

## Bounded continuation request

“Read HANDOFF.md and docs/09-wrap-up-plan.md in my rfile repository. Resume only
R7 Linux confinement after confirming the R6c checkpoint is published. Review
the sandbox spec and CLI ordering, implement and verify the next bounded increment. Preserve named compatibility decisions. Update the checkpoint with
completed tests and remaining gaps before moving to another milestone.”

This scopes authorized maintenance; it is not an access-control workaround.
For repeated product restrictions, preserve files, record the exact notice and
use the official reporting guidance linked in the plan.
