# rfile restart checkpoint

Updated 2026-10-31 UTC (overnight October 30–31 PDT). The user authorized
release preparation overnight with independent GPT-6 Luna testing agents after
Claude exhausted its quota. Codex fixes; independent agents test and retest.
The endpoint is an experimental default-feature x86_64 Linux/glibc CLI and
vendored source. Publication requires the maintainer's final approval.

## Frozen candidate and GitHub state

- Repository: https://github.com/JLuethe/rfile.
- Release preparation [PR #23](https://github.com/JLuethe/rfile/pull/23) merged
  with all checks passing. Frozen release source:
  `e64ce034bee6a6c0959872d9b8f632d692dbfaa3`.
- Its Git tree `91506a94437571b6fc3257f1d2e860824e19b727` exactly equals
  `a781c017b3495121a4203053cbabe55225cb0130`, the independently tested commit.
  Reports retain actual executed identities; squash-merge equivalence is recorded
  separately. Later documentation/evidence commits do not change the release target.
- Final package workflow: https://github.com/JLuethe/rfile/actions/runs/41178070592.
- PR #19 merged after independent queue QA. PRs #20 and #1 closed; optional
  dependency PRs #21/#22 remain explicitly deferred. Both Linux and MSRV checks
  are required, including for administrators.
- Existing public v0.1.0 remains unchanged. No new release is published.
- CLI requires GLIBC 2.34 symbols; the oldest tested runtime is Ubuntu 22.04
  (glibc 2.35), not every glibc 2.34 distribution. Source MSRV remains Rust 1.87.
  Additional codecs, confinement and C interface remain experimental source options.

## Independent findings and validation

Initial Claude reports are preserved in `reports/release-review-2026-10-30/`
and `reports/release-readiness-2026-10-30/`. Luna independently reviewed the
queue, exact FILETIME undefined-reference boundary, and ZIP64 helper variants.
The artifact review found and closed fresh-cache rustc discovery and missing
system timezone failures through Codex fixes and independent retests. Historical
rejected archives and infrastructure failures remain recorded, not relabeled.

Both pinned-oracle CI profiles, Linux/MSRV checks, packaging and 13 minimal-image
checks passed on a781. Independent full-source and exact final artifact acceptance passed and are
recorded under the final evidence directory below. A bounded
25-target ASan replay passed with 75.771719 measured CPU seconds; Miri passed two
tests. No target met its 24-CPU-hour qualification gate.

## Completed delivery and recovery paths

The [unpublished release draft](https://github.com/JLuethe/rfile/releases/tag/untagged-c161acd97cb00e0a1c09)
targets e64 and holds the exact accepted Ubuntu CI archives. The
[final acceptance record](reports/release-0.2.0-rc.1/final/README.md) contains
checksums, source equivalence, independent reports and scope limitations.
Default/all-feature verification passed 32/34 stages; strict sandbox checks
executed in debug/release. Fixture coverage is 103 files / 1,339 truncations.
Rust 1.87, 15 Python regressions and 582/582 valid source dumps passed.
Exact archives passed 14 package, 10 installer and 13 minimal-Ubuntu checks.
Two clean local builds were reproducible on the same host/toolchain; their source
matches CI, but cross-host binary reproducibility is not claimed.

Durable evidence/scratch, if the thread resumes:

- `.reference/release-final-e64/`: delivery checkpoint, assets, release notes and
  final raw evidence. Consult `CHECKPOINT.md` before restarting any work.
- `.reference/qa-checkouts/a781c01`: source-test checkout. The old
  `/tmp/rfile-final-acceptance-a781c01` symlink preserves cached fixture paths.
- `.reference/release-final-e64/evidence/source-qa/rfile-final-acceptance-a781c01-qa/`:
  runner/checkpoint and command logs, including failed prerequisite attempts.
- `.reference/package-qa-e64/`: accepted exact archive QA, full report and recovery
  instructions. The older `.reference/package-qa-0f/` preserves rejected artifacts.
- `.reference/release-rc1-sanitizers/qa-evidence/candidate-a781c01/`: actual
  25-target ASan and two-test Miri execution with command and binary hashes.
- `.reference/package-repro-e64ce03/`: two clean final builds and comparison.

No production blocker found by these release checks remains open. The maintainer
still reviews and publishes the draft; no public release is authorized. Do not
rebuild or substitute accepted assets without new checksums and retesting.
Future qualification remains separate: at least 600 measured CPU-hours across
25 targets, coverage/resource evidence and independent unsafe/resource review.
Optional dependency PRs #21/#22 and other-platform certification remain deferred.

## Reference / resources

The primary oracle is `/home/john/Code/rfile/.reference/file-clean`, clean commit
`41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. Always set RFILE_REFERENCE explicitly.
The old `upstream/` retains a local seccomp patch; never reset it or call it clean.
`.reference/cdf-pointer` is an explicitly patched auxiliary oracle only.
Cargo is `/home/john/.cargo/bin/cargo`. Default/all-feature runs must be serialized
within a checkout. Use separate checkouts and CARGO_BUILD_JOBS=1 while other builds
run; /tmp is RAM-backed, so large temporary builds should use scratch on disk.
Rootless Podman works for disposable unprivileged offline runtime checks; the
Docker socket does not. Do not change global configuration to work around that.

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

## Interruption recovery

[Recovery specification](docs/10-execution-and-recovery-spec.md) remains the
contract. No permanent agent auto-restart service is installed. Check the live
agent/process inventory and each isolated checkout's actual queue manifest before
restarting work. A surviving process is not proof of a completed test; retain its
exit status and logs. Do not duplicate a queue or combine campaign accounting
across changed source/binary/supervisor identities.

Continuation: read this checkpoint, release scope and independent QA reports;
continue the experimental release through fixes, independent retests, final
artifact acceptance and an unpublished draft. Codex fixes; independent testing
agents test/retest. Preserve all uncompleted security gates explicitly.
