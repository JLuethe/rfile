# rfile restart checkpoint

Updated 2026-10-24. Read [the wrap-up plan](docs/09-wrap-up-plan.md), then the
relevant detailed specification. This is a checkpoint, not permission to publish
a release or a promise of active background work.

Read [execution and recovery](docs/10-execution-and-recovery-spec.md) for R6c
refinements and future supervisor requirements. Recovery automation is specified,
not installed. Security restrictions require review, not automatic resubmission.

## Current state

- Repo: https://github.com/JLuethe/rfile; local `/home/john/Code/rfile`.
- Reviewed implementation main: `25f4135f1144528cca89a4cfc5954e2ccacf067f`.
  CDF, ELF/core, codecs, regex increment and locale increment are merged.
- Research branch `feat/format-time-edges`, pushed commit
  `463de0953b61b62a7f3b317ee1df2a3af95c5c13`, is not merged. It adds only
  `tools/float_compare.py` and `reports/float.json`: 6,630 cases, 4,078 exact,
  2,552 differences. No formatter repair yet; script intentionally fails.
- Latest main evidence: `reports/verification-extended.json`, 24 passing stages,
  113 ordinary debug/release tests. Those runs predate this documentation review.
- Sandbox, C ABI, sustained campaigns, independent review and other-platform
  certification remain unfinished. Extra codecs stay opt-in.
- No build/fuzz worker was running when checked during the review. Recheck;
  this statement is historical, not live supervision.

## Resume

1. Inspect `git status --short --branch` and recent history; fetch origin.
   Preserve local changes. The local branch named main was behind at review;
   do not confuse it with the current active branch or origin/main.
2. For implementation, resume `feat/format-time-edges`, incorporating current
   main normally. Inspect `src/format.rs`, `src/time_context.rs`, the float
   harness and `docs/remaining/04-semantics.md`.
3. Review the research harness, add focused regressions and fix bounded format
   semantics. Distinguish source rejection from compiled runtime behavior;
   record pinned/corrected decisions. Never broadly allowlist 2,552 failures.
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
R6c formatter/date compatibility on feat/format-time-edges. Review the failing
baseline, implement bounded Rust fixes and verify against the pinned local
oracle. Preserve named compatibility decisions. Update the checkpoint with
completed tests and remaining gaps before moving to another milestone.”

This scopes authorized maintenance; it is not an access-control workaround.
For repeated product restrictions, preserve files, record the exact notice and
use the official reporting guidance linked in the plan.
