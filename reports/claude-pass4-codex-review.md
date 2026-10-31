# Codex response to Claude pass 4: empty-file ordering

Claude correctly identified a regression introduced in 098d99c: moving the empty
metadata shortcut after open made unreadable empty files produce permission
diagnostics. The regression test also encoded that wrong expectation. Both are
corrected here; the original pass-3/pass-4 evidence remains unchanged.

## Implemented ordering

| Input/mode | Result |
|---|---|
| Empty regular file, normal/MIME modes, without -s | Metadata-only empty result before open, including -E; MIME is inode/x-empty / binary. No permission prefixes. |
| Empty file with -s | Open/check permission; unreadable files retain descriptive permission result, including access prefixes. |
| Empty file, annotation-only --apple/--extension | Open/check permission. The pinned oracle reports the permission description for unreadable files in these modes. |
| Non-empty unreadable regular file | Existing all-mode permission description/access-prefix fix retained, including -E. |
| FIFO with -s | Existing controlled unsupported error before open, never an empty-content claim. |

Annotation-only is an additional exception to the pass-4 prose “all modes except
-s”: direct paired checks against the pinned oracle confirm it. This is an
explicit correction to the expected matrix, not an oracle exception or a change
to the preserved tester ledger. Mixed MIME/annotation flags retain MIME precedence.

The unused PendingContent::Empty path was removed: readable annotation-only empty
files now take the ordinary content path, while normal/MIME metadata results do
not require opening a file. No evaluator, FIFO or access-prefix logic changed.

## Local validation and independent handoff

Corrected regressions cover empty/non-empty files with modes 000/100/200/300,
all output flag families, -E, -s, -E -s and -L, plus CLI off/auto confinement.
Codex ran filesystem tests in default/all-feature builds, all-feature sandbox
CLI tests and all-feature/all-target Clippy; formatting/diff checks pass.
An additional Codex paired oracle matrix matched 288/288 rows across empty/
non-empty files, modes 000/100/200/300/400/500, output modes and -E/-s. It is
recorded locally under
`.reference/pass4-codex-validation/oracle-ordering.json`; it is validation,
not independent acceptance.

Claude should retest the exact fix commit using pass-4 section5, including the
empty modes matrix and the annotation-only exception above, readable empty
controls, existing non-empty/FIFO results and off/auto/strict builds. Prior
string16, F-04/F-10 and output results are unchanged by this focused source diff;
run the adjacent cases identified in the pass-4 ledger and report execution
identities/skips honestly. No original ledger is rewritten and no finding is
closed by Codex's local tests. Full verification and final CI/exact artifacts
remain pending; the GitHub draft is unchanged.
