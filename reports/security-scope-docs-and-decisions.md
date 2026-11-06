# Security-scope brief: documentation corrections and decision status

Source brief, preserved unchanged:
`/home/john/Code/rfile-qa-claude/security-scope/SECURITY-REVIEW-BRIEF.md`.
This records the requested section-9 items 1–3 corrections and the status of
section-8 decisions. It is not a security review or campaign result.

## Documentation corrections

1. **Release scope:** the active accepted candidate is ca9465f, tree
   92b34ed5c86ea5737a4eb1f00e3ce085fe59dca5, artifact 13091831221. Claude's current
   independent passes, including 6–11, are credited to Claude. Earlier Luna work
   and the e64 draft retain their historical identities and are labeled historical.
2. **Fuzz inventory:** fuzz/README.md now says 25 targets, matching the 25 bin
   entries read from fuzz/Cargo.toml. This static inventory does not claim a new
   build, review, coverage result or campaign execution.
3. **Audit status:** reports/dependency-audit.json is annotated as historical and
   not applicable to the frozen lock. Its original result is preserved byte for
   byte in dependency-audit-2026-10-25.raw.json; existing provenance is unchanged.
   The raw result's SHA256 remains
   `4bd5f1adbefc6e93e99316142b9490ae8e304910b9618b9c7fdc6fc6c6bffe44`.
   Its audited workspace-lock SHA256 was
   `1b8077fb97322634635d3f4963b9d9c0f52ee0bad3eec485644f90cbe69b0096`.
   The frozen workspace lock is
   `c7a1d2b995471286b3b89b3cfa5c18ece793d0a87c23c5e03ab0e6f2155f3f12`,
   and the frozen fuzz lock is
   `28a0f365cd93ac3fb8f18abb300d5ddd7e40b4014986e113be6e1c4fe41d2cc3`.
   Those hashes are recorded separately; the historical audit was not relabeled
   as if it tested the new lock. The separate October 31 release audit provenance
   does match the frozen locks, but remains date-bound historical evidence.
   A fresh current-advisory audit has not been run in this change.

## Section 8: what the maintainer has decided

The conversation authorizes scoping and these documentation corrections and
explicitly forbids starting review or fuzz execution now. It does **not** contain
explicit approval of the brief's proposed review plan or new choices for items
8(a)–(h). Existing repository policy remains in force; it is distinguished from
new decisions below. Codex recommendations and Claude assumptions are not approvals.

| Item | Decision status / existing baseline |
|---|---|
| (a) Shipped-default-only first review, optional WP-9/10 later | Pending explicit review-scope selection. The accepted downloadable binary is already default-only; that does not authorize a particular review tier. |
| (b) Fresh cargo-audit with advisory-DB network access | Not authorized or run. The current instruction is documentation only. |
| (c) Severity/disposition policy for DoS against caps | Pending. The brief's proposed policy has not been adopted. I-1/I-2's earlier delegated documentation disposition is separate. |
| (d) Hours per target versus per seed | Written baseline: at least 24 measured CPU-hours per target and at least two seeds; 25 targets give a 600 CPU-hour minimum. No new seed-allocation clarification or 1,200-hour requirement is approved. |
| (e) Campaign host / dedicated VM | Pending host selection. Existing isolation/accounting requirements do not constitute permission to start a host or campaign. |
| (f) External audit beyond agent review | Not commissioned or authorized in this conversation. |
| (g) Confidentiality channel | SECURITY.md already requires private GitHub vulnerability reporting. No new channel choice or policy change is recorded; follow that existing policy. |
| (h) New hostile-input deployment guidance in release notes | Pending wording/scope decision. Existing experimental/security statements are unchanged. |

The compatibility [maintainer disposition worksheet](remaining-maintainer-dispositions.md)
also remains pending; producing or correcting the scope brief does not approve
its recommendations. No new approval is requested as a prerequisite to finishing
this documentation task.

## Boundaries

The accepted candidate remains `ca9465f5f93eca75c80352a18a5de108e20762a0`, tree
`92b34ed5c86ea5737a4eb1f00e3ce085fe59dca5`. These later documentation/evidence
annotations do not alter its archives, regenerate an audit or retarget its CI
identity. No production code, dependency lock, workflow, campaign configuration,
review result or Claude evidence was modified. No security review, advisory DB
fetch, fuzzing, build, CI dispatch, publication, tag or draft-asset change occurred.
