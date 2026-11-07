# Historical overnight handoff and independent testing plan

Prepared 2026-10-31 from the repository's recorded evidence. rfile is a Rust
file-type detector based on file/libmagic behavior, not a `less` pager. This is
a dated handoff; its older candidate identities and tester instructions below
are preserved as history, not current release directions.

**Current status:** [v0.2.0-rc.1](https://github.com/JLuethe/rfile/releases/tag/v0.2.0-rc.1)
is published from `7d3e700db435289e7b5d8774895a39293bf94152` with newly
built binary and vendored-source archives. The release includes the manifest,
checksums and QA evidence; the exact published archives passed package,
minimal-runtime and offline-source checks. Three repository security advisories
are published with that version as fixed. See [release scope](release-scope.md)
for the current feature boundary. Sustained fuzz qualification and an independent
security audit remain open. Later development commits do not change the tag.

## What happened overnight

- Codex implemented release-readiness fixes and packaging. Claude performed the
  initial baseline and retests; after its quota ran out, independent GPT-6 Luna
  agents continued testing and retesting. The actual reviewer and tested commit
  are retained in each report.
- Fixes made required fixture and sandbox coverage visible and fail closed,
  preserved verification runs and failures, corrected fresh-Cargo-home tool
  discovery, and made an absent implicit system timezone work in minimal images.
  FILETIME oracle undefined behavior and exact ZIP64 helper differences received
  narrowly scoped, independently tested handling.
- PR #19 added the bounded fuzz queue. PR #23 merged the release preparation at
  `e64ce034bee6a6c0959872d9b8f632d692dbfaa3`; PR #24 merged final evidence and
  documentation. Main at handoff is `3c39353866a13ed4eb9802c36917cae699d073a4`.
- The release source's tree `91506a94437571b6fc3257f1d2e860824e19b727` is
  identical to tested commit `a781c017b3495121a4203053cbabe55225cb0130`. The
  execution reports retain `a781` as their tested commit; tree equality does not
  turn those executions into runs performed at `e64`.
- Independent source verification passed 32 default and 34 all-feature stages,
  including actual debug/release sandbox checks. Each mutation run covered 103
  fixtures, 1,339 truncations and 30,000 deterministic mutations. Rust 1.87,
  15 Python tests and 582 valid source dumps passed. The exact final archives
  passed 14 package, 10 installer and 13 minimal Ubuntu runtime checks, plus an
  offline source build and raw-byte filename checks.
- A bounded ASan replay exercised 25 targets for 75.771719 aggregate CPU seconds;
  two Miri tests passed. This is smoke evidence, not sustained qualification.
  Two same-host builds were identical; their source archive matched CI, while
  their binary archive differed from CI. Cross-host binary reproducibility is
  not established.
- Initial ptrace/LeakSanitizer, temporary-storage quota and relocated-cache
  failures were preserved separately from successful retries. A non-UTF-8 test
  failure was traced to Podman's argument transport; direct binary and
  in-container shell tests passed.
- An unpublished prerelease draft then contained nine verified assets and
  targeted `e64`; its downloaded assets were
  checked against their local bytes and GitHub digests. The existing v0.1.0
  release was not changed at that checkpoint. The draft assets were later
  replaced by the published `7d3e700` release.

Read [HANDOFF.md](../HANDOFF.md), the [release scope](release-scope.md), and
the [final acceptance record](../reports/release-0.2.0-rc.1/final/README.md)
before starting. The latter links the independent verdicts and hash records.
Raw evidence is attached to the draft and also under
`.reference/release-final-e64/`. Local `.reference` and `/tmp` paths are
machine-specific and may not survive a move or reboot.

## Current remaining qualification work

There are two distinct endpoints:

1. **Experimental GitHub release:** completed at the published tag above. The
   binary imports GLIBC 2.34 symbols and was tested on Ubuntu 22.04/glibc 2.35;
   other distributions and platforms are not certified. Optional codecs,
   sandbox and C ABI remain experimental source options, not included binaries.
2. **Stronger security-qualified release:** [SEC-01 through SEC-10](remaining/06-hardening.md)
   remain open where evidence is missing. At least 24 measured CPU-hours per
   mandatory target across 25 targets means **600 CPU-hours minimum**, plus at
   least two seeds per target, coverage/resource evidence, final-candidate replay,
   triage and independent unsafe/resource review. A smoke test, wall-clock hours
   or a green queue is not completion. Fixes or material dependency changes
   require affected campaign reruns under the documented identity rules.

Optional dependency PRs #21 (rustix) and #22 (lzma-rust2) were deferred at this
handoff. Their later disposition belongs in the current [release scope](release-scope.md)
and GitHub PR history. Other-platform certification, optional binary variants,
known compatibility boundaries and a security audit remain separate work.
Do not use this dated handoff to infer current GitHub state or move the published
tag. The following tester instructions describe the earlier prepublication plan.

## Archived instructions for the independent tester

1. Record date, model, host/tool versions, `git status`, commit and tree. Inspect
   the draft and existing reports; distinguish executed evidence, skips,
   infrastructure failures and untested claims. Identify whether the subject is
   the frozen release commit or newer main. Do not relabel old runs.
2. Use an isolated clean clone or worktree and separate build directories; do
   not change the shared checkout, production source, the clean reference, or
   the draft. Serialize default and all-feature runs within each checkout. Use
   disk-backed scratch and `CARGO_BUILD_JOBS=1` when resource contention is a
   concern; `/tmp` was RAM-backed during the overnight work.
3. Verify the pinned oracle before use: the recorded clean checkout is
   `/home/john/Code/rfile/.reference/file-clean`, commit
   `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`. Set `RFILE_REFERENCE` explicitly.
   The separate `upstream/` directory contains a local patch; do not reset it or
   represent it as the clean oracle. If prerequisites or isolation are missing,
   record the block rather than disabling checks or treating skips as passes.
4. Independently review the scope, previous finding closures and exact archive
   hashes. Exercise untested or weak boundaries first: malformed/truncated
   source and databases, CDF/ELF offsets, compression budgets, regex and output
   limits, cancellation/IO faults, non-UTF-8 paths, timezone failures, FFI
   ownership and actual sandbox enforcement where applicable. Use synthetic or
   redistributable inputs, bounded resources and separate native-oracle processes.
5. Run the relevant verifier on the *identified* candidate with its prerequisites
   satisfied. For a full capable Linux runner, from the isolated checkout:

   ```sh
   export PATH="/home/john/.cargo/bin:$PATH"
   export RFILE_REFERENCE=/home/john/Code/rfile/.reference/file-clean
   export CARGO_BUILD_JOBS=1
   python3 tools/verify.py --benchmark
   python3 tools/verify.py --all-features --require-sandbox --benchmark
   python3 -m unittest discover -s tools -p 'test_*.py' -v
   cargo +1.87.0 check --workspace --all-targets --all-features --locked
   ```

   Read per-run reports and logs, confirm required fixture counts and actual
   sandbox execution, and preserve hashes. Run the commands sequentially.
   Without fresh `RFILE_ORACLE_REPORTS`, the report-specific Python checks may
   skip; report that honestly. LeakSanitizer cannot run under ptrace on the
   recorded host: use an appropriate runner, preserving failures and retries.
6. Test the exact proposed binary and source archives, rather than substituting
   a local rebuild. Check `SHA256SUMS`, manifest and source commit; exercise
   install/uninstall, empty-Cargo-home offline rebuilding and the supported
   runtime. Record image digest and actual glibc version. If a new candidate is
   built, require fresh hashes and independent retesting of its exact bytes.
7. For sustained security work, first read SEC-04 and the queue's `--help`,
   inspect existing processes/locks/manifests and define the candidate, isolation,
   resource limits, seeds, budget, evidence storage and stopping conditions.
   Use the existing isolated supervisor; never bypass bubblewrap or aggregate
   CPU across changed identities. Record partial work as partial. A full campaign
   is a long-running resource commitment, not a prerequisite to reporting the
   already-scoped experimental draft as experimental.
8. Deliver a dated findings ledger and verdict, including exact commands,
   source/tree, features, toolchain, inputs, expected/actual bytes and exit
   status, minimized synthetic reproducer, logs/hashes, severity and acceptance
   criteria. Separate product defects, intended documented differences,
   infrastructure failures and skips. Do not edit expectations to hide failures.
   Preserve raw evidence outside tracked source and provide recovery instructions.

## Codex review and fix loop

1. Read the tester's entire report and raw evidence; verify source identity and
   independently reproduce each actionable finding. Triage severity, release
   impact and scope. Investigate disputed results, including whether the launcher
   or the native oracle caused them, before changing product behavior.
2. Codex implements focused fixes with meaningful regression coverage and runs
   appropriate local checks. Preserve previous findings and failed-run evidence.
   Do not broaden oracle exceptions or alter expected outcomes without a specific
   documented compatibility decision.
3. Give the independent tester the fix commit and original reproducer. The
   tester reruns the reproducer and adjacent boundaries, records the new evidence
   and either closes or reopens each finding. Codex reviews that retest and runs
   remaining validation. Repeat until no in-scope critical/high defects remain;
   document and obtain a maintainer disposition for any remaining lower findings.
4. A source, dependency, feature or packaging change may invalidate the frozen
   draft. Determine affected tests and security campaigns, build a new immutable
   candidate, rerun them, independently test its exact artifacts, and update
   hashes, notes and evidence. Never silently substitute assets or attribute old
   execution to a new commit. Present the concrete revised draft for review.
5. Update the handoff and final verdict with commands, actual outcomes, unresolved
   gates and recovery paths. Publication and any stronger security claim require
   the appropriate maintainer decision and completed gates.

## Prompt to give Claude or another independent testing agent

```text
You are the independent testing agent for rfile, a Rust file-type detector.
Codex implements fixes; you test, document findings, and retest fixes. Do not
edit production code, publish releases, change tags, alter draft assets, or
silently modify expected results. You may create test-only reproducers and
evidence in an isolated checkout/scratch directory.

Read docs/12-overnight-handoff-and-agent-plan.md, HANDOFF.md,
docs/release-scope.md, docs/remaining/06-hardening.md, and
reports/release-0.2.0-rc.1/final/README.md. Check current Git, GitHub and
advisory state rather than assuming the dated handoff is current. Record your
model, commit/tree, environment, feature flags, commands and hashes. The frozen
draft targets e64ce034bee6a6c0959872d9b8f632d692dbfaa3; previously tested
a781c017b3495121a4203053cbabe55225cb0130 has the identical recorded tree but
must retain its own execution identity.

Assess what remains for the experimental draft separately from the longer
security qualification. Inspect prior findings and evidence, then design and run
bounded independent tests for the most consequential uncovered boundaries and
the exact candidate artifacts. Follow the detailed tester instructions in the
handoff. Use the explicitly pinned clean oracle, isolated checkout, disk-backed
scratch, actual sandbox enforcement and reproducible synthetic inputs. Preserve
all failures and distinguish product defects, oracle/launcher issues,
infrastructure failures, skips and known scope limitations. Do not count smoke
fuzzing as 600 CPU-hours or claim an independent security audit has occurred.

Produce a prioritized ledger with exact reproduction commands, input, expected
and actual bytes/status, logs, source identity, severity and acceptance
criteria; include a clear pass/fail/blocked verdict for each scope and list any
unexecuted gates. Keep evidence and a restart checkpoint. Send findings to Codex
for review and fixes. Once given a fix commit, independently rerun the original
reproducer and adjacent cases, report the new identity and evidence, and close
only findings actually verified. Continue this tester/fixer loop until the
requested scope is complete or a concrete external dependency blocks it.
```
