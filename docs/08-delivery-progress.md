# Remaining-work delivery progress

Tracking: https://github.com/JLuethe/rfile/issues/3. Specification: [remaining work](07-remaining-work-spec.md). Spec PR #2 is merged. This ledger distinguishes implemented evidence from release gates.

| Milestone | Current state | Evidence / next gate |
|---|---|---|
| R0 new discrepancy coverage | implemented, CI pending | `tools/component_compare.py`: 5 comparisons, 3 exact and 2 exact-baseline known-open discrepancies; changed results/errors fail rather than inherit a broad ignore. Two Rust controls pass; two explicitly ignored regressions retain the missing expected behavior. |
| R1 shared safety contracts | next | common IO/budgets/errors/profile/capability implementation and focused boundary tests |
| R2 immediate semantic repairs | pending | explicit backreference detection and local dates |
| R3–R6 format/codec/full semantics | pending | detailed component acceptance matrices |
| R7–R8 sandbox and C ABI | pending | real denied-operation tests and all 19 ABI functions/client contracts |
| R9 sustained hardening | pending | measured campaigns, final-candidate replay and independent review |
| R10 compatibility release | pending | all declared release gates and remaining gaps reviewed |

The existing baseline reports remain unchanged. `reports/components.json` separately records the additional CDF/regex cases. Ignored regression tests and known-open discrepancies are not counted as successful conformance. No sustained campaign or independent audit has been performed by this PR.
