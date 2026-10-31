# rfile restart checkpoint

Updated 2026-10-31 UTC (overnight October 30–31 PDT). The user authorized
release preparation overnight with independent GPT-6 Luna testing agents after
Claude exhausted its quota. Codex fixes; independent agents test and retest.
The endpoint is an experimental default-feature x86_64 Linux/glibc CLI and
vendored source. Publication requires the maintainer's final approval.

## Pass-4 correction awaiting Claude retest

Code candidate `8e33173217dd89b20c227d7665319681114ee9cb`, tree
`b20a51c44a44e42bf65bdde5e11ee4a10ac34ad7`, on local branch
`fix/claude-timezone-diagnostic`. See
[Codex pass-4 review](reports/claude-pass4-codex-review.md).

Restores metadata-first empty-file identification in normal/MIME modes, including
-E. Both -s and annotation-only --apple/--extension must open/check permissions;
the annotation exception was confirmed directly against the pinned oracle.
Non-empty unreadable diagnostics, access prefixes and explicit FIFO -s errors
are retained. The earlier PendingContent::Empty annotation shortcut is removed.

Local default/all-feature filesystem regressions, sandbox CLI tests, Clippy and
formatting passed. A Codex paired matrix matched 288/288 rows across empty/
non-empty files, modes 000–500 and output/-E/-s combinations. This is local
validation, not independent acceptance; Claude has not retested this commit.
Retest per `/home/john/Code/rfile-qa-claude/pass4/LEDGER-pass4.md` section5 with
its expected normal/MIME ordering and the verified annotation-only exception.

Original ledgers remain unchanged. The remaining F-04/F-04b/F-03/F-07/CDF-stage
dispositions and final release gates remain open; the old e64 draft is unchanged.
The earlier pass-3 candidate notes below are historical and superseded here.

## Earlier pass-3 candidate and implementation notes

Earlier code candidate: `098d99cc837a07e1d40b79c6e96acd97f00caa1e`, tree
`d3fe580ecaceee719b8d82a61cc51b79abe5a8c1`, local branch
`fix/claude-timezone-diagnostic`. See
[Codex pass-3 review](reports/claude-pass3-codex-review.md).

F-08 now preserves unreadable regular-file descriptions and access prefixes in
all output modes, including -E; checks open permission before identifying empty
files; and rejects FIFO -s content inspection before open. String16 wildcard
reads beyond EOF synthesize an empty value safely (F-10 padding candidate).
General shifted-use bounds remain strict and are documented (F-04); invalid CDF
geometry stage differences, F-03 headline caveat and F-07 release-note limitations
are disclosed. F-04b remains unroot-caused; F-03/F-07 dispositions remain pending.

Local targeted default/all-feature filesystem/semantics/sandbox CLI checks and
Clippy passed. Independent Claude retest has NOT run on this commit. Pass-3
ledger/checkpoint and section9 scripts are in `/home/john/Code/rfile-qa-claude/pass3/`.
Run its filesystem/FIFO/paired string16/CDF matrices against this exact commit,
using isolated unprivileged runners and bounded owned-process cleanup. No new
archives or final release gate are claimed. The old e64 draft is unchanged.
Earlier results below retain their executed identities and do not qualify 098d.

## Claude follow-up fix results (2026-10-31)

Local branch: `fix/claude-timezone-diagnostic`. The earlier pass-2 code candidate was
`aafff27664ee7c6f91da76af8ca6a86f960235bc`; the F-01/F-02/F-09 batch below
records the latest candidate. See the
[retest index](reports/claude-pass2-fix-retest/README.md) and original Claude
ledger `/home/john/Code/rfile-qa-claude/pass2/LEDGER-pass2.md`.

- F-03: prior diagnostic/documentation changes at 3903a19/8343a33 passed exact
  local archive testing as a documented limitation; maintainer acceptance pending.
- F-05/F-06: d7d0eca fixes passed Claude's 28 offset probes, 284-case compiled
  parity matrix, byte comparisons and bounded scaling (D200k: 0.12 CPU seconds,
  124 MB RSS versus about 19 seconds before). Both full verifier profiles passed
  32/34 stages at d7, using a recorded private clean build of the same oracle pin.
- F-07: d7 plus 2ef0bbc/aafff27 follow-ups fix output panics, read-only stdout,
  quiet EPIPE, debug/status stderr failures, and preserve silent compilation.
  Claude's final focused default/all-feature and strict-sandbox tests passed at
  aaff. Inherited-closed stdout remains an explicitly documented open criterion.

Execution reports retain actual source identities. Full d7 results and focused
final aaff results do not claim a full verifier execution at aaff. Original
failed environment attempts are preserved. No tester processes remain running.
Other ledger findings and final finding dispositions remain open.

Raw evidence and recovery: `.reference/claude-pass2-retest-evidence/`,
`.reference/claude-debug-retest-evidence/`, `.reference/claude-final-output-evidence/`.
The old e64 draft remains unchanged and does not contain these fixes. Before
replacement, resolve/dispose remaining findings, freeze the final candidate,
run its required release checks and independently accept exact CI-built archives.
The original completion record below concerns the old experimental draft and
must not be read as closure of the newly discovered findings.

## F-01 / F-02 / F-09 follow-up (2026-10-31)

Code candidate `2bed7f803c5ca1fbcab2ae7b6feea2ec20adfdb2` on the same local
branch passed Claude's independent retest; see the
[batch report](reports/claude-f01-f02-f09-retest/README.md).
F-01 signed ELF position/read-span diagnostics match all 5,880 expanded cases
and 5,461 standard ELF comparisons. F-09 expected refusals and malformed
manifest/receipt validation pass on the packaged installer. F-02's delegated
choice is to retain/document the complete-decode hard cap with configurable
`bytes`, rather than introduce prefix sampling; boundary/workaround/integrity
checks pass on the exact archive binary. Package checker: 14/14.

Raw evidence and recovery: `.reference/claude-2bed7f8-evidence/`; intermediate
failures and corrections remain in `.reference/claude-ec687fc-evidence/` and
`.reference/claude-f01-f02-f09-evidence/`. No tester processes remain running.
Full final verify.py/CI release artifacts were not run for this batch. The old
e64 draft is unchanged. Other findings, final release gates and the security
qualification endpoint remain open as described above.

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
