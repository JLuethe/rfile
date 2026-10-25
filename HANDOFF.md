# rfile restart checkpoint

Updated 2026-10-25. Read [the remaining plan](docs/07-remaining-work-spec.md),
[delivery ledger](docs/08-delivery-progress.md), and the relevant component spec.
This records evidence, not a completed security qualification or release.

## Repository and reference

- R7 PR #15 and the experimental C ABI PR #16 are merged. Typed diagnostics PR #17 is also merged. Latest reviewed main:
  `51a31affc81c1068d8b2f94f2d3e67ddd8d2c624`. Linux/MSRV checks passed.
- Active branch: `feat/cli-source-actions`. It integrates native CLI database
  action output, missing-file provenance and bounded regular-file reads.
  Inspect Git/GitHub for its latest commit/PR before resuming or publishing.
- The primary clean oracle is `.reference/file-clean`, exact upstream commit
  `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. Always use
  `RFILE_REFERENCE=/home/john/Code/rfile/.reference/file-clean`.
  The old `upstream/` has a preserved local seccomp patch; never reset it or call
  it clean. `reports/reference-audit.json` records identities and the correction.
- `.reference/cdf-pointer` is an explicitly patched auxiliary oracle only.
  Ignored binaries, toolchains, corpora and logs are not supplied by a fresh clone.

## Tested current increment

- All 33 stages of `tools/verify.py --all-features --benchmark` passed:
  145 ordinary debug/release tests; 143 isolated default tests; Rust 1.87;
  unchanged v21 database and matching benchmark checksum (warm ratio 1.607).
- ABI state: 49/49 original scenarios and 440/440 expanded diagnostic scenarios
  exact. The six previously frozen diagnostic differences are now resolved.
  Strict Rust source constructors retain their existing first-error behavior.
- 582/582 valid source-check dumps pass. Seven new ordinary source regressions
  include stale continuation state and rejecting oversized Pascal-string records
  before compilation could emit an unimportable record.
- Two applicable FFI Miri tests pass with `LC_ALL=C TZ=UTC0` and
  `MIRIFLAGS='-Zmiri-env-forward=LC_ALL -Zmiri-env-forward=TZ'`.
  Miri does not establish host timezone/locale/kernel correctness.
- 25 ASan fuzz targets build. Historical R8b short isolated runs: 519,332 diagnostic
  cases / 30.120558 CPU seconds; 156,904 ABI sequences / 30.932193 CPU seconds.
  No sustained target has met its 24-CPU-hour gate. Review actual live manifests
  and process identities before launching duplicate work.
- PR #16's isolated package passed dynamic/static clients, default discovery and
  an offline vendored-source rebuild (`reports/abi-package.json`, code 62d7b13).
  That package is historical and must be rebuilt for a later release candidate.
  cargo-audit had no findings at its recorded RustSec revision; independent
  review has not occurred. No global installation/system replacement/new release.

## Next actions

1. Reconcile Git, current reports, running processes and GitHub checks. Preserve
   edits. Checkpoint this increment through a PR; require Linux and MSRV success.
2. CLI actions now have 243 exact cases across off/auto/strict modes. Continue
   [remaining actions](docs/remaining/07-c-abi.md): default discovery, DEBUG, broader
   accepted warnings/IO behavior and ordinary classification source loads.
3. Expand accepted native warning/format/conditional/directory/IO error fixtures;
   unmapped diagnostics remain explicit gaps. No universal drop-in claim.
4. Continue R9 isolated measured campaigns, coverage and resource/ownership review.
   25 targets require at least 600 measured CPU-hours for the minimum full gate;
   partial overnight work cannot substitute. Independent review is separate.
5. Rebuild and verify final packages only against the tested candidate; retain
   Linux-only and native regex/locale/CDF/codec limitations in release statements.

Cargo is `/home/john/.cargo/bin/cargo`. Default/all-feature CLI tests must be
serialized or use isolated `CARGO_TARGET_DIR` to avoid replacing each other's
CLI during tests. Fuzz supervisors use bubblewrap, target locks and inner wait4
CPU accounting. Never combine campaigns across source/binary/supervisor hashes.

## Interruption recovery

[Recovery specification](docs/10-execution-and-recovery-spec.md) remains the
contract. No agent auto-restart service is installed. A security/content notice
requires review, not an automatic retry loop. Local builds may continue after a
chat disconnect; inspect their logs/PIDs instead of assuming completion or death.

Continuation: “Read HANDOFF.md and the remaining-work specs. Reconcile source,
workers and GitHub. Continue the next bounded ABI/CLI task, then hardening.
Preserve compatibility decisions and checkpoint actual results.”
