# Remaining maintainer dispositions after pass 11

Candidate: `ca9465f5f93eca75c80352a18a5de108e20762a0`, tree
`92b34ed5c86ea5737a4eb1f00e3ce085fe59dca5`; independently accepted CI artifact
13091831221. This is a decision worksheet, not an approval or release verdict.
Claude's original ledgers and compatibility documentation remain authoritative
for detailed reproducers and limits.

For each row, the maintainer can accept the documented contract for the
experimental scope, request a Codex change, or defer the affected scope. Every
decision below is still **pending**; the recommendations do not close findings.

| Item | Verified behavior / remaining difference | Codex recommendation | Decision |
|---|---|---|---|
| F-03 explicit unavailable TZ | Missing named-zone data stops the entire run, including MIME-only/non-date input. UTC works; the diagnostic names TZ. Unlike glibc, no silent UTC fallback. | Accept documented configuration requirement for experimental scope. | Pending |
| F-07 output handling | Rust startup may reopen inherited-closed stdout as /dev/null, allowing quiet success. EPIPE exits quietly with status 1 instead of native SIGPIPE behavior. | Accept documented process/output contract; do not add a pre-main runtime hook solely for parity. | Pending |
| F-04 shifted-use bounds | Rfile checks absolute read bounds. The oracle can match synthesized past-end values inside use blocks; 240 golden rows differ, unchanged across accepted profiles/artifacts. | Retain strict bounds and documented difference. | Pending |
| F-10 invalid CDF geometry | Invalid sector shifts can be rejected at a different stage with different diagnostics. | Accept documented behavior on malformed geometry. | Pending |
| String16 non-wildcard residual | 208 matrix cases involving =\0 / <z differ at tested lengths 2, 3, 12 and 13; wildcard and source/compiled parity are verified. | Retain the narrow wildcard exception; defer broader emulation until the rule is justified. | Pending |
| Mixed flags / F-08d | -i --apple emits one selected/target result rather than the oracle's concatenated metadata and annotation output, including regular symlink targets. | Accept documented selection instead of oracle concatenation. | Pending |
| Directory symlink annotations | Controlled unsupported diagnostic differs from native Is-a-directory wording and mixed-flag prefix. | Accept documented diagnostic. | Pending |
| FIFO content requests | -s returns a controlled unsupported error without opening/blocking; metadata mode identifies FIFO. Native error text differs. | Accept explicit unsupported-content contract. | Pending |
| Unreadable/empty files with -E | Metadata-first empty identification and descriptive unreadable results follow the recorded ordering; -s and annotation-only requests check content permissions first. | Accept tested/documented ordering. | Pending |

I-1/I-2 are **resolved as documented-intentional for the pass-11 archive bytes**
under the user's delegated fix-or-document choice. F-04b and F-08c's fixed
annotation-only subset retain their independent verification; they are not new
decision requests here. Previously delegated choices, such as the documented
decompression hard cap, are not reopened by this worksheet.

The default experimental CLI scope and the stronger security-qualified endpoint
remain distinct. A security-review scope brief, actual independent review,
coverage/resource evidence, 600 measured CPU-hours under the existing full-target
protocol and final-candidate replay are not fulfilled by accepting compatibility
differences. Publication remains a separate maintainer action.
