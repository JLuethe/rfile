# rfile restart checkpoint

Updated after candidate CI/archive production following Claude pass 9. The user authorized
release preparation overnight with independent GPT-6 Luna testing agents after
Claude exhausted its quota. Codex fixes; independent agents test and retest.
The endpoint is an experimental default-feature x86_64 Linux/glibc CLI and
vendored source. Publication requires the maintainer's final approval.

## Candidate CI passed; exact archives await independent acceptance

The requested candidate `3f9af8d89dcf2990a842cf6e4e6e9c1b28a0c228`, tree
`f08995301414e2976b70f6e289fc58f00a05b62d`, is pushed on the dedicated branch
`ci/rc1-pass9-3f9af8d`. Required linux/msrv checks passed in run
[41272111258](https://github.com/JLuethe/rfile/actions/runs/41272111258).
Release-candidate packaging passed in run
[41272131089](https://github.com/JLuethe/rfile/actions/runs/41272131089), including
14 package/offline-rebuild checks and 13 minimal-runtime checks. Both runs and
the remote branch bind the exact requested commit/tree.

[CI artifact 13069527384](https://github.com/JLuethe/rfile/actions/runs/41272131089/artifacts/13069527384)
is downloadable until 2026-11-15T04:13:06Z. See
[the completed CI handoff](reports/ci-3f9af8d-handoff.md) for every artifact hash,
download commands, packaged binary identity, source binding and preserved logs.
Raw evidence is retained under `.reference/ci-pass9-3f9af8d/`, with a checksum
inventory. The downloaded ZIP hash matches GitHub's digest; both tarball hashes
match their manifest. Claude/maintainer independent exact-archive acceptance is
next and has not been inferred from CI self-checks. No publication, tags or draft
asset replacements occurred. This checkpoint update is documentation after the
fixed build target; the candidate branch remains pinned.

## Pass-9 sweeps passed; next CI/archive acceptance (historical)

Tested source remains `8325cefd5d3c8c7d22015d8a97c948196330c435`, tree
`b6e431e70d286914acc5d52417982e99b36151de`. Claude's original pass-9 evidence
is at `/home/john/Code/rfile-qa-claude/pass9/LEDGER-pass9.md` and is unchanged.
Default and strict all-feature golden sweeps covered 103 fixtures, 4,696 derived
inputs and 14,088 comparisons per profile. The five F-04b rows are resolved;
the 240 documented F-04 residual rows are unchanged, with no new differences.
The string16 1,200-case matrix confirms wildcard behavior and source/compiled
parity; the 208 non-wildcard residuals remain. Both binaries reproduce F-03's
documented limitation in the pinned minimal Ubuntu container. No product defects
were reported.

Compatibility wording is corrected: the tested string16 =\0/<z residual occurs
at lengths 2, 3, 12 and 13, not only two-byte inputs. This update changes docs
only; source-verifier evidence remains attributed to the tested source commit.

Next gate: build fresh CI candidate archives from the new immutable docs-inclusive
commit, then have Claude independently accept those exact downloaded bytes,
including hashes/provenance, installer behavior and offline source rebuild.
Record maintainer dispositions for the documented differences before publishing.
Independent security review, coverage/resource evidence and the 600 CPU-hour
campaign remain open for security qualification; review should precede the long
campaign so any resulting code fixes can be included in its frozen candidate.
Existing draft assets remain unchanged. Older checkpoint sections are historical.

## Pass-8 source verifiers passed; next independent sweep (historical)

The source candidate remains `8325cefd5d3c8c7d22015d8a97c948196330c435`, tree
`b6e431e70d286914acc5d52417982e99b36151de`. Claude pass 7 verified F-04b fixed
and F-08d documentation accurate; pass 6 verified F-08c's annotation-only subset.
Pass 8 ran both full source-verifier profiles sequentially on this candidate:
31/31 default steps and 33/33 all-feature steps passed. Required sandbox CLI and
kernel tests executed in all-feature debug/release stages. This meets the local
source-verifier criterion on the reported host, without claiming release or
security qualification.

Codex reviewed the structured reports and Cargo logs and verified all 133 evidence
inventory hashes. Reporting correction: each Cargo test stage logs two deliberate
caught panics in passing containment/unwind tests; "no panic messages" is too
broad. Original ledgers remain unchanged. See
[the pass-8 evidence review](reports/claude-pass8-codex-review.md).

Next: [Claude pass-9 prompt](reports/claude-pass9-test-prompt.md) for the full
golden truncation sweep, string16 matrix and F-03 minimal-container repro on the
same candidate. Maintainer dispositions, CI-built archives/exact-asset acceptance,
sustained fuzzing, coverage/resource evidence and independent security review
remain open. No source, draft, tag or asset changes accompany this update.
Older checkpoint sections below are historical and retain their original scope.

## Pass-6 candidate awaiting Claude retest (historical)

Fix commit `8325cefd5d3c8c7d22015d8a97c948196330c435`, tree
`b6e431e70d286914acc5d52417982e99b36151de`, on local branch
`fix/claude-timezone-diagnostic`. See
[Codex pass-6 review and retest instructions](reports/claude-pass6-codex-review.md).

F-04b now selects plain UTF-8 using decoded non-ASCII characters, preserving
incomplete-tail tolerance after completed characters, BOM handling and the
one-byte very-short path. F-08d is documented in compatibility notes, man page
and release notes: mixed -i --apple on readable/empty/chained regular symlink
targets emits only the target result, without the oracle's symlink MIME prefix.
No filesystem output behavior changed in this candidate.

Local default/all-feature semantic tests pass (29 each), as do Clippy and
formatting. The 24 pass-6 F-04b probes across three flag sets match the oracle in
72/72 comparisons. Claude's independent retest of this candidate remains pending.
Claude pass 6 verified F-08c's annotation-only unreadable symlink subset at e8cdcb4;
mixed-flag, directory-wording and FIFO dispositions remain with the maintainer.
Original evidence, draft and assets remain unchanged. All final verification and
security gates remain open. Earlier checkpoint sections below are historical.

## Pass-5 candidate awaiting Claude retest (historical)

Fix commit `e8cdcb409edfcab152eb5c411fd08567834b2c3a`, tree
`c3337de53018900f5ba7c4430352c99a1cbc205d`, on local branch
`fix/claude-timezone-diagnostic`. See
[Codex pass-5 review and retest instructions](reports/claude-pass5-codex-review.md).

Claude's pass-5 ledger independently verified the earlier 8e33173 empty-file
ordering fix in 324 rows per build/profile; no findings were closed. The new
candidate fixes annotation-only unreadable symlink diagnostics and documents
mixed-flag concatenation and non-regular-target differences. F-04b is now
root-caused in a preceding report-only commit; encoding code remains unchanged.
Non-wildcard string16 behavior is documented without extending the evaluator.

Local focused tests and Clippy pass. A Codex paired 324-row matrix has 10
differences versus 18 before, eight newly exact matches and no newly differing
rows. This is not independent acceptance or packaged-artifact verification.
Original Claude ledgers/evidence, the old draft, tags and release assets remain
unchanged. All maintainer decisions and final verification/security gates remain
open. The pass-4 and older sections below are historical records superseded by
this candidate and the original pass-5 ledger.

## Pass-4 correction awaiting Claude retest (historical)

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
