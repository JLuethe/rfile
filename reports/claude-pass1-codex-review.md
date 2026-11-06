# Codex review of Claude pass 1

Claude's original ledger and raw evidence are in
`/home/john/Code/rfile-qa-claude/LEDGER.md` and `repro/`; the evidence inventory
was checked with `sha256sum -c EVIDENCE-INVENTORY.sha256 --quiet`. The tester
executed at `e64ce034bee6a6c0959872d9b8f632d692dbfaa3`. Findings remain open
until independently retested or explicitly disposed by the maintainer.

| Finding | Review and disposition |
|---|---|
| F-03 | Preserve the existing explicit-zone error contract; make the error identify the configured `TZ` and document the no-tzdata limitation and workarounds. This improves the diagnostic and disclosure; it does **not** implement UTC fallback or defer the error until a date is rendered. Claude must retest and decide whether this satisfies the documented-limitation option. |
| F-02 | Bounded decompression hard-fails at the limit; `-P bytes=N` can raise it within other limits. Behavioral difference remains open for a compatibility decision or documentation and boundary regression. |
| F-01 | Out-of-range ELF note behavior is fail-closed but differs from native output; retain as open pending a focused compatibility decision and regression. |
| F-06 | Quadratic rule output is a resource concern for caller-supplied rules, not a completed security review. Retain as open for profiling, budget/algorithm review and independent bounded retest. |
| F-04 | Native zero-padding behavior versus strict EOF remains an open compatibility decision; do not imply exact equivalence. |
| F-05 | Reproduced Claude's three-rule case: rfile prints `w2 w4`, pinned oracle prints `w2`. Relative-base semantics need investigation and a regression before a safe fix. |
| F-07 | Reproduced exit 0 with closed stdout. Remains open for a portable output-error fix and pipe/closed-descriptor regression. |
| F-08 | Unreadable-file and FIFO differences remain open for documented compatibility disposition. |
| F-09 | Installer traceback on refusal remains open as a user-facing diagnostic issue; refusal itself was reported safe. |
| F-10 | CDF malformed-header diagnostics and padding remain open compatibility differences. Claude's generator mistakes were retracted and are not product findings. |

The F-03 change creates a new source snapshot. It does not alter the existing
unpublished draft, which still contains the original e64 binary and does not
include the new diagnostic or revised notes. Do not publish that draft on the
assumption it includes this change. Freeze a new candidate, rebuild and test the
exact archives, and update the draft only after independent acceptance. Full
security qualification, including sustained campaigns, remains open.
