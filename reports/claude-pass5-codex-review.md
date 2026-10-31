# Pass-5 Codex review and exact-commit retest handoff

Date: 2026-10-31. Codex owns fixes/documentation; Claude owns independent
testing and finding closure. Read the original pass-5, pass-4 and pass-3 ledgers
in that order. They and their raw evidence were not modified.

## Candidate identity

- Fix commit: `e8cdcb409edfcab152eb5c411fd08567834b2c3a`.
- Tree: `c3337de53018900f5ba7c4430352c99a1cbc205d`.
- Local branch: `fix/claude-timezone-diagnostic`.
- Earlier investigation-only commit:
  `d55baf7eb910dca580cee35b12d56af72c4d3c8f`, tree
  `d2df3019cb2213f75858408968c606aa90db1074`.

This report and the accompanying HANDOFF update are a subsequent documentation
commit. Retest the fix commit above, not an inferred latest HEAD. No candidate
archive is supplied: Claude should build that exact commit in an isolated clone.

## Fixed and documented

**F-08c:** annotation requests without `-L` now retain regular symlink-target
metadata and open the target before using an empty-file shortcut. Unreadable
empty/non-empty targets produce the permission description and the applicable
writable/executable prefixes. Regression expectations for `l_e000` and `l_n000`
come from the pinned oracle: `regular file, no read permission` for `--apple`,
`--extension`, `--apple --extension` and `-s --apple`.

Mixed `-i --apple` also inspects the symlink target and now emits its permission
description. It deliberately emits that description only once: the oracle
prefixes it with `inode/symlink; charset=binary`. This remaining mixed-flag
difference is documented, not claimed fixed to oracle bytes.

**Mixed flags on direct empty files:** documentation only, as requested. Rfile
emits the MIME result once. More precisely than the pass-5 summary, the oracle
concatenates the metadata MIME result with an empty-content MIME result when
readable, or a permission description when unreadable. No direct-empty ordering
or MIME-selection code was changed.

**Directory symlink annotations:** retain the controlled unsupported error.
The difference from `cannot read ... (Is a directory)` is now explicit in the
compatibility notes and release notes. Rejecting non-regular annotation targets
before opening them preserves the bounded special-file handling.

**String16 residual:** document the non-wildcard `=\0` / `<z` empty-comparison
differences rather than generalizing the earlier wildcard exception. The
evaluator was not changed; an oracle rule justifying that extension has not been
established.

Regression fixtures now use a timestamp nonce as well as PID, avoiding stale
fixture collisions when sandbox PIDs are reused.

## F-04b: root cause reported, encoding unchanged

The investigation was committed before this production change. See
[the root-cause report](claude-pass5-f04b-investigation.md).

Rfile tolerates an incomplete UTF-8 tail by decoding the valid prefix, but then
selects UTF-8 using high bytes in the original input, including discarded tail
bytes. The oracle requires at least one completed multibyte character for plain
UTF-8 detection. This explains both two-byte fixtures and the 11-byte matilde
`-k` case; completed-character controls agree. A future narrow fix should use
decoded non-ASCII characters while preserving incomplete-tail tolerance after
completed characters and the separate BOM/parser behavior. No encoding code was
changed: F-04b remains open for the next agreed fix/retest cycle.

## Codex local validation, not independent acceptance

Passed:

- `cargo test --locked --test filesystem_diagnostics`: 3 tests.
- `cargo test --all-features --locked --test filesystem_diagnostics --test sandbox_cli`:
  3 filesystem tests and 1 sandbox CLI test.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- `cargo fmt --all -- --check` and `git diff --check`.

Ran Claude's unchanged pass-5 `tools_matrix.py` locally against the all-feature
debug binary with `--sandbox=off`, `LC_ALL=C`, `TZ=UTC0`, and an explicit pinned
oracle database. Result: 324 rows, 10 differences (previously 18), 8 newly exact
matches, no newly differing rows versus Claude's pass-5 default results. The
remaining differences are the eight direct-empty mixed-flag rows and two
mixed-flag symlink rows described above. This does not cover root/ACL/other-owner
permissions, and is not Claude's retest.

Local evidence: `.reference/pass5-codex-investigation/f08c-allfeatures-off.json`
(SHA256 `5485e40f806e9bf042c625f92fe16e71827616bf6fd0b56dd4963e9af127e820`).
The locally built debug binary SHA256 was
`a4a640cf50254943a66797d482371e0113e9dfa5c095cf77cc503e32cc89205e`;
it is not a packaged or CI artifact.

## Claude retest request

1. Record the exact commit/tree above, build default and all-features binaries
   in a clean isolated clone, and record commands and binary hashes.
2. Rerun pass-5 `tools_matrix.py` in the default profile and all-feature
   `--sandbox=off/auto/strict` profiles. Compare with pass-5 evidence. Expect the
   eight annotation-only symlink rows to newly match; keep the ten documented
   mixed-flag differences separately identified. Check readable and dangling
   symlink controls and the directory-target controlled error.
3. Retain the empty/unreadable ordering matrix and bounded FIFO checks as
   adjacent regression coverage. Confirm documentation against actual outputs.
4. Independently assess the F-04b explanation using the two-byte ARJ/PDF and
   11-byte matilde `-k` cases, plus complete/incomplete UTF-8 controls. Expect
   unchanged outputs; this candidate does not implement an encoding fix.
5. Write a new ledger/checkpoint/inventory, preserving all earlier evidence.
   Close only what you independently verify and leave maintainer decisions open.

## Still open

No finding is marked closed here. F-08c's fixed subset awaits Claude retest;
mixed flags, directory wording and the existing FIFO contract have documented
dispositions for maintainer review. F-04b needs its separate code fix. String16
non-wildcard residuals remain open. F-03 unavailable explicit TZ, F-07 inherited
closed stdout/quiet EPIPE, F-10 invalid sector-geometry error stages and F-04
shifted-use bounds remain maintainer decisions, without inferred acceptance.

Neither verifier profile was run on this candidate. CI-built archive acceptance,
exact release-asset acceptance, the 600 CPU-hour fuzz campaign, coverage/resource
evidence and independent security review remain outstanding. The old e64 draft,
release assets, tags and pinned oracle were not changed; nothing was published.
