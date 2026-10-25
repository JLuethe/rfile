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
- Active branch is `feat/c-api-diagnostics`, R8 prerequisite work. A bounded
  byte-preserving `Database::legacy_list` renderer and focused listing and source-check tests are
  implemented. Initial comparison: 5/5 listing stdout results match the clean
  reference, but four source-load stderr warnings are missing. A bounded source
  check-dump primitive now matches 221 valid synthetic programs and all 361
  selected upstream source fragments (582/582 successful dump cases); malformed
  warning rendering and complete loader actions are still open. This is research,
  **not completed action/ABI conformance**; see reports/r8a-listing-baseline.json.
- C ABI exports do not exist yet. Next: source-load origins/diagnostics, complete
  check/list action ordering/state, then all 19 functions and client/ownership/
  packaging verification per docs/remaining/07-c-abi.md. That spec's header line
  reference was corrected to magic.h.in:128–162 during this review.
- No build/fuzz worker remains active at this checkpoint. Native quantified
  histories, broader semantics/platforms, sustained campaigns and independent
  review remain open. No global install or new release has been performed.

## Resume

1. Inspect Git/workers/GitHub before repeating any action; preserve local edits.
   R7 is already merged. Rebase the R8 branch onto origin/main if not yet done.
2. Use `RFILE_REFERENCE=/home/john/Code/rfile/.reference/file-clean` for oracle
   tooling. `upstream/` is the preserved modified historical checkout.
3. Continue R8 source diagnostics/listing. Trace `apprentice_1`,
   `file_apprentice`, `file_magwarn`, `file_mdump`, and source parser outcomes.
   Do not confuse matching listing stdout with matching stderr/state or the ABI.
4. Run focused differential and error/state/ownership tests; once implementation
   stabilizes, run the full matrix and native client/packaging checks. Serialize
   default/all-feature integration tests or use separate target directories:
   building one feature set can replace a CLI still used by another test run.
5. Update docs/evidence, push a PR, require both hosted checks, then proceed to
   the next milestone. Keep feature delivery separate from security qualification.

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
