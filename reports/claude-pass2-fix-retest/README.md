# Claude independent retest of pass-2 fixes

Code candidate: `aafff27664ee7c6f91da76af8ca6a86f960235bc`, tree
`2239dad4fc66e33d8dbd0938863ca2a9594fbfcb`.

Codex implemented fixes; Claude independently tested them. Original pass-1/pass-2
ledgers remain unchanged under `/home/john/Code/rfile-qa-claude/`.

| Finding | Result and remaining boundary |
|---|---|
| F-05 | Fix passes all 28 probes and the 284-case source/compiled matrix. Defined sticky end-offset behavior follows the pin with checked arithmetic. >2 GiB oracle/int32 behavior and some additional option combinations remain untested. |
| F-06 | Fix passes measured acceptance: D200k 0.12 CPU seconds versus 19.2 before, same 124 MB RSS; A100k 0.07 seconds. Outputs match across tested families and 70 NUL/RAW/nested-use cases. Wider use-heavy scaling remains untested. |
| F-07 | Partial: printing panics, read-only stdout, broken-pipe messaging and unwritable debug/status stderr are addressed. Inherited-closed stdout still becomes /dev/null before main and exits 0; that criterion remains open. EPIPE policy is quiet status 1. |

These are verified fix results; formal finding disposition remains with the
maintainer. F-03 documented-limitation acceptance and the other ledger findings
remain open. No release was published, merged or updated by this retest.

## Execution identity and scope

The [main report](claude-main-fix-report.md) and [default](verification-default.json)
/ [all-feature](verification-allfeatures.json) verifier reports executed at
`d7d0eca9d5da7f0a867e6ea95aab505840d398c8`: 32/34 stages passed, including required
actual debug/release sandbox coverage and 103 fixtures / 1,339 truncations.
A private clean build of oracle commit `41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59`
was recorded with its own hashes; configuration header and rule database match
the shared primary oracle. Earlier environment failures are preserved separately.

The [debug retest](claude-debug-report.md) executed at `2ef0bbc`; the
[final output retest](claude-final-output-report.md) executed at `aafff27`, including
both feature profiles and actual strict sandbox enforcement. They establish the
affected diagnostic/write-path behavior after d7; they are not full verifier
runs on aaff. Execution identities are never relabeled.

## Recovery and remaining release work

Raw main evidence/checkpoint/hash inventory: `.reference/claude-pass2-retest-evidence/`.
Final output evidence: `.reference/claude-final-output-evidence/`.
Debug evidence: `.reference/claude-debug-retest-evidence/`.
Main evidence SHA256 inventory was verified by Codex after Claude returned.

The old e64 GitHub draft still contains the older binary. Final CI-built archives
of the chosen new candidate need exact-byte acceptance before draft replacement.
No final aaff archive or whole final-candidate release verification is claimed.
Remaining findings, sustained fuzzing, coverage/resource evidence, independent
unsafe review and other-platform certification are separate unfinished work.
