# rfile restart checkpoint

Updated 2026-10-30. Start with [release scope and PR decisions](docs/release-scope.md),
then [the remaining plan](docs/07-remaining-work-spec.md),
[delivery ledger](docs/08-delivery-progress.md), and the relevant component spec.
This records evidence, not a completed security qualification or release.

## Repository and reference

- Current release-preparation branch: `fix/release-readiness-baseline`, based on
  `7a3ea756e79838e4c3ca252d23c3e9ff7ab8fe76` (PR #19 head). Main was
  `5150a1ac832902cc1e866930ff2b2d45d3ef841a` when rechecked.
- PR #19 is still open with passing Linux/MSRV checks; independent queue review
  remains before merge/qualification. PR #20 was closed because it replaced the
  declared Rust 1.87 minimum check. Other dependency/action PRs are explicitly
  deferred in the release contract. No new release or dependency upgrade occurred.
- The user authorized steps 1 and 2: Claude tests/retests, Codex fixes and performs
  other validation. Scope is a default-feature x86_64 Linux/glibc CLI and source
  release. Packaging and final candidate/version freeze remain subsequent work.
- The primary clean oracle is `.reference/file-clean`, exact upstream commit
  `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. Always use
  `RFILE_REFERENCE=/home/john/Code/rfile/.reference/file-clean`.
  The old `upstream/` has a preserved local seccomp patch; never reset it or call
  it clean. `reports/reference-audit.json` records identities and the correction.
- `.reference/cdf-pointer` is an explicitly patched auxiliary oracle only.
  Ignored binaries, toolchains, corpora and logs are not supplied by a fresh clone.

## Release-readiness increment

- The independent baseline in `reports/release-review-2026-10-30/` passed 143
  default and 145 all-feature tests, formatting, all-feature Clippy, a release
  build and two CLI smoke commands. One mutation test was ignored.
- Claude then reproduced a false coverage success: an explicit reference with
  103 fixtures yielded zero fixture truncations. The fix honors the reference,
  requires fixtures by default and records explicit opt-outs.
- Sandbox tests now emit structured coverage and can require execution.
  Verification runs preserve per-run logs, source identity and failure/interruption
  status, replacing the latest summary at startup. Check the follow-up independent
  QA report before treating this changed implementation as retested.

## Historical 2026-10-25 validation

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
- 25 ASan fuzz targets build. A resource-constrained all-target smoke completed
  81.238734 measured CPU seconds, separately archived in fuzz-queue-all-smoke.json.
  Duplicate-job rejection, partial-run status and deadline cleanup pass.
  No sustained target has met its 24-CPU-hour gate. Review actual live manifests
  and process identities before launching duplicate work.
- Historical isolated packages passed dynamic/static clients, default discovery
  and an offline vendored-source rebuild. The latest source-provenance report
  records `1198c0e` and 966 matching files. Those artifacts must be rebuilt and
  retested for a later release candidate.
  cargo-audit had no findings at its recorded RustSec revision; independent
  review has not occurred. No global installation/system replacement/new release.

## Next actions

1. Read the release contract and independent follow-up QA evidence. Finish PR
   #19's queue review/self-test and release-readiness fixes before final candidate
   freeze. Build and verify the default CLI/source packages on a clean host.
   No overnight campaign was started by this increment.
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
contract. No agent auto-restart service is installed. The local fuzz queue is implemented;
its latest actual job is recorded in ignored `.reference/fuzz-queues/latest.json`.
Inspect the linked queue.json and user-systemd status before claiming it is active. A security/content notice
requires review, not an automatic retry loop. Local builds may continue after a
chat disconnect; inspect their logs/PIDs instead of assuming completion or death.

Continuation: “Read HANDOFF.md, docs/release-scope.md and independent QA evidence.
Continue the experimental CLI release preparation; Claude tests/retests and Codex
fixes plus validates. Preserve the separate security qualification gates.”
