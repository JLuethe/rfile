# Remaining-work delivery progress

Tracking: https://github.com/JLuethe/rfile/issues/3. Specification: [remaining work](07-remaining-work-spec.md). Spec PR #2 is merged. This ledger distinguishes implemented evidence from release gates.

| Milestone | Current state | Evidence / next gate |
|---|---|---|
| R0 new discrepancy coverage | merged in PR #4 | `tools/component_compare.py`: 5 comparisons, 3 exact and 2 exact-baseline known-open discrepancies; changed results/errors fail rather than inherit a broad ignore. Two Rust controls pass; two explicitly ignored regressions retain the missing expected behavior. |
| R1 shared safety contracts | implemented, local suite passed; hosted CI pending | positional IO and tracked buffers, shared work/IO/memory budgets, cancellation/deadlines, typed resource errors, semantic profiles and machine-readable capabilities; 13 new boundary tests |
| R2 immediate semantic repairs | pending | explicit backreference detection and local dates |
| R3–R6 format/codec/full semantics | pending | detailed component acceptance matrices |
| R7–R8 sandbox and C ABI | pending | real denied-operation tests and all 19 ABI functions/client contracts |
| R9 sustained hardening | pending | measured campaigns, final-candidate replay and independent review |
| R10 compatibility release | pending | all declared release gates and remaining gaps reviewed |

The baseline reports are regenerated for each implementation candidate. `reports/components.json` separately records the additional CDF/regex cases. Ignored regression tests and known-open discrepancies are not counted as successful conformance. No sustained campaign or independent audit has been performed by this PR.

## R1 accounting contract

One synchronous inspection owns thread-confined work counters; child/decompression/evaluator passes share them. Concurrent calls on a shared `Engine` create independent budgets. Cancellation uses a cross-thread atomic token, and memory reservations can be dropped on another thread. No process-global locale or limits change. Limits setters are transactional even when repairing an already invalid configuration.

`InputView` performs checked positional IO. File reads preserve the borrowed cursor; partial reads return only observed bytes. Repeated interruptions count toward IO requests. `InspectionBytes` reserves replacement capacity before growth and releases leases on error/drop. IO counters conservatively count requested bytes, including decoder reads; they are not a syscall profiler.

Memory metrics measure tracked buffers and conservative workspace envelopes, not allocator overhead or process RSS. Encoding, legacy CDF, result accumulation and regex compiler/Pike scratch receive explicit reservations. The immutable database/shared regex cache has separate caps. Continuing format work must migrate legacy nested allocations to individual reservations; R1 does not certify a universal allocator-enforced memory ceiling. Cooperative cancellation is checked at IO/work boundaries and every 1024 evaluator steps; an in-progress bounded dependency operation can finish before the next check.

`--profile=pinned-5.48` is the default; `corrected-v1` selects only implemented, documented corrections as they arrive. `--capabilities` reports actual current support and false security-review/fuzz gates. Selecting a profile is not evidence that all planned profile semantics exist.

R1 validation: `tools/verify.py --benchmark` passed all 19 verification stages, including 103 fixtures, 22,544 broad API comparisons, 11,000 extended comparisons, 10,875 rule cases, exact v21 compiler bytes, 31,339 mutation cases and the five component probes (two still known-open). The 50 ordinary Rust tests pass; three ignored tests remain separate (two planned discrepancies and the separately executed mutation campaign). CLI deviations retain the existing six named cases. Warm benchmark output checksums match; median Rust/C ratio is 1.83 on this machine. No sustained fuzz or independent review gate was executed.
