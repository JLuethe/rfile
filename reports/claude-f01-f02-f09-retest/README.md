# F-01, F-02 and F-09 implementation and independent retest

Code candidate: `2bed7f803c5ca1fbcab2ae7b6feea2ec20adfdb2`, tree
`dd43679f5e7dda58846301c209bb2d17c6b23f7f`. Codex fixed code; Claude designed and
executed independent tests. No GitHub draft, tag or asset was changed.

| Finding | Resolution |
|---|---|
| F-01 | PASS in tested scope. Invalid signed ELF note/program-section positions and read spans now produce native-style diagnostics without an invalid read. All 5,880 expanded comparisons match, and the existing 5,461-case ELF suite passes with the source-built and exact archive binaries. |
| F-09 | PASS. Expected installer errors are concise; explicit schema validation rejects malformed manifests and receipts before filesystem changes. Symlink/refusal, rollback, malformed metadata, permissions and preservation checks pass on the packaged installer. |
| F-02 | PASS as a documented deliberate difference. Keep the complete-decode hard cap (7,340,032 bytes by default). Limit errors do not return partial inner/wrapper descriptions. `-P bytes=N` raises the cap within other budgets. Prefix sampling requires an explicit incomplete-result contract and is deferred. |

## Decisions and evidence

The user delegated F-02's choice to Codex. Retaining complete decoding preserves
checksum/truncation handling rather than implying integrity from an unread tail.
Manual, compatibility policy and release notes describe the cap and workaround.
Claude tested gzip/zlib cap minus one/exact/plus one, -z/-Z, path/stdin, raised
caps, corruption/truncation and 1 GiB bombs on the exact final binary.

[Final Claude report](claude-final-report.md),
[package check](package-check.json), [artifact manifest](release-manifest.json).
The package check passes all 14 checks, including an offline source rebuild.
Claude also passed affected Rust ELF/codec suites in dev/release/extended-codecs
and the four Python installer tests. Codex ran targeted tests, formatting and
all-feature/all-target Clippy. Full verify.py on this final commit was not run.

The [intermediate report](claude-intermediate-report.md) preserves the additional
F-01 cases Claude found at ec687fc; Codex fixed them at 2bed7f8 and Claude
retested. Earlier e0f testing found malformed-manifest issues fixed at ec687fc.
The superseded e0f test process was intentionally stopped (exit143); its partial
results are not a passing verdict. Earlier raw ledgers/evidence remain untouched.

Tester harness corrections are explicit: an invalid `files=[]` receipt must
refuse with status1, and CLI classification errors appear on stdout with status1.
Original incorrect-expectation logs were preserved beside corrected copies.

## Hash review and recovery

Raw final evidence/checkpoint:
`.reference/claude-2bed7f8-evidence/`. Earlier stages:
`.reference/claude-ec687fc-evidence/` and `.reference/claude-f01-f02-f09-evidence/`.
Codex checked both tester inventories: every recorded file matched except the
live `session.jsonl`, which gained CLI shutdown/result events after hashing.
Original inventories were preserved; separate POSTRUN-INVENTORY.sha256 files
record the completed streams and evidence. No test-result file mismatch occurred.
The Claude report's assertion that source/archive binary differences are *only*
embedded paths was not independently established; both distinct hashes are
recorded and tested, without a binary-equivalence/reproducibility claim.

## Remaining scope

Other ledger findings and prior documented limitations remain as recorded.
The e64 draft still contains older binaries. A final chosen candidate still needs
required release verification/CI and independent exact CI-artifact acceptance
before draft replacement and maintainer publication review. No security-qualified
release, sustained fuzz gate, other-platform certification or audit is claimed.
