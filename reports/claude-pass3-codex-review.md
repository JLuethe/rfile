# Codex implementation handoff for Claude pass 3

Original ledger: `/home/john/Code/rfile-qa-claude/pass3/LEDGER-pass3.md`.
Original reports and reproducer mistakes/retractions remain unchanged. This is
an implementation/local-validation record, not independent finding closure.

## F-08: implemented

- An existing regular file whose open is denied returns the pinned permission
  description in every output mode, including MIME, annotations and -E.
  Writable/executable prefixes use the real-user access queries used by the
  pin, implemented at the narrow Linux/glibc platform boundary. Actual stat and
  other open errors retain their prior -E handling. This compatibility decision
  treats known unreadable metadata as a result, not an unsuccessful inspection.
- Empty-file identification now follows the open check, so an unreadable empty
  file cannot bypass permission diagnostics.
- FIFO content inspection (-s) returns a controlled unsupported error before
  opening it. Default FIFO metadata behavior remains. No queued-content emptiness
  claim, blocking FIFO read, native null diagnostic or SIGSYS emulation is added.

## F-04 / F-10: bounded compatibility and disclosure

String16 wildcard reads (lestring16/bestring16, relation x) beyond EOF now
synthesize an empty string without any out-of-range access. Other types and
non-wildcard patterns retain their bounds checks; source/compiled regressions
cover this narrow exception. General shifted-use reads remain absolute-bounds
checked. Documentation explains explicit and end-relative use positions, the
observed gzip/ZIP/PNG/APNG/PGP/zstd scope and limited MIME evidence.

Invalid CDF sector geometry may fail at a different parsing stage. That is now
explicitly documented rather than described as mere wording. F-04b very-short
text encoding remains open and unroot-caused. No sweep was rerun or relabeled.

## F-03 / F-07 disclosure

The release headline now includes the explicit-zone/tzdata caveat; the detailed
limitation still requires maintainer disposition. Release notes now disclose
quiet EPIPE and inherited-closed stdout. The manual qualifies its inability to
recover the pre-startup descriptor state as applying to checks after runtime
startup; no pre-main hook or signal-policy change was added. Those disposition
items remain open.

## Local checks and Claude retest

Codex ran default filesystem/semantics regressions, all-feature filesystem,
semantics and sandbox-CLI tests, and all-feature/all-target workspace Clippy plus
format/diff checks. Permission tests require an unprivileged runner, check empty
and nonempty files, modes 000/100/200/300, output flags and -E/-s/-L; CLI tests
also cover off/auto confinement. FIFO test ensures -s is an explicit error.

Claude should build the exact commit in an isolated checkout and run pass-3
section 9 commands: F-08 matrix and bounded FIFO writer/no-writer cases; F-04
paired controls and string16 source/compiled comparisons; F-10 corrected CDF
matrix; release-note checks for F-03/F-07. Exercise both feature profiles and
strict sandbox where supported. Do not count intentional shifted-use, invalid
CDF geometry, or explicit FIFO-unsupported outcomes as native exactness. Record
those differences and verify the chosen documented contract instead. Preserve
all failures; correct harness expectations only with explicit justification.

The pass-3 ledger lists the timezone container repro both as executed and in its
skip list. Consult raw logs or rerun it on the new candidate; do not infer a fresh
run from the contradictory summary. The invalid first F-04 output was overwritten
by the tester and cannot be recovered by relabeling later runs.

Check for owned leftover FIFO test processes before starting; use bounded child
cleanup, never broad pkill. Keep all findings open until verified/disposed.
No final verify.py, new archive, CI artifact or stronger security gate is claimed
for this batch. The GitHub draft still contains old e64 assets.
