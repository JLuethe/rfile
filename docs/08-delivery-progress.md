# Remaining-work delivery progress

Tracking: https://github.com/JLuethe/rfile/issues/3. Specification: [remaining work](07-remaining-work-spec.md). Spec PR #2 is merged. This ledger distinguishes implemented evidence from release gates.

| Milestone | Current state | Evidence / next gate |
|---|---|---|
| R0 new discrepancy coverage | merged in PR #4 | `tools/component_compare.py`: 5 comparisons, 3 exact and 2 exact-baseline known-open discrepancies; changed results/errors fail rather than inherit a broad ignore. Two Rust controls pass; two explicitly ignored regressions retain the missing expected behavior. |
| R1 shared safety contracts | merged in PR #5 | positional IO and tracked buffers, shared work/IO/memory budgets, cancellation/deadlines, typed resource errors, semantic profiles and machine-readable capabilities; 13 new boundary tests |
| R2 immediate semantic repairs | implemented, local verification passed; hosted CI pending | checked immutable TZif/POSIX contexts; local/UTC families and explicit corrected FILETIME; decoded regex lexer rejects backreferences and Rust-only groups |
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

## R2 semantics and dependency boundary

Rust callers default to UTC and can provide an immutable `TimeContext`; CLI database/cookie initialization snapshots TZ, including named system zones, POSIX expressions and explicit TZif files. Files are capped at 1 MiB; named resolution rejects traversal outside the canonical zoneinfo root. Linux uses nonblocking open before checking the handle is a regular file. No global environment, libc timezone or locale state is changed. Capabilities include the chosen timezone provenance.

Pinned FILETIME deliberately retains both the existing calendar normalization and the newly measured standard-time `mktime` interpretation (`tm_isdst=0`), while the corrected profile uses signed ticks and the actual 1601 epoch. `MAX_CTIME` is the pinned constant `0x3afff487cf`, not its approximate comment. Jiff excludes a small interval at the top of its timestamp range; a checked 400-year Gregorian recurrence handles the boundary only beyond the final explicit TZif transition. Unsupported extreme custom transition tables and perpetual-DST standard-time inference remain edge cases for R6 rather than a claim of universal libc equivalence.

The new date differential contains 1,022 exact cases spanning endian/width families, negative/2038/year9999 boundaries, FILETIME calendar boundaries, DST, half/quarter-hour offsets and seven TZ configurations, with TZif hashes and glibc version. Current locale display is still C. Other locale/float/dialect edge matrices remain R6.

Recognized decoded backreferences produce a visible error in both source loading and imported compiled database evaluation. The component manifest records this exact, normalized error as `explicit-unsupported`, separately from exact matches. It is safer than the former silent nonmatch, but remains a conformance gap until the bounded VM lands; its expected successful-match regression remains ignored. Escaped backslashes and digits within bracket predicates do not trigger the error. Non-ERE `(?...)` extensions are rejected explicitly.

New dependency: jiff 0.2.38 with only std/tz-fat, plus jiff-core 0.1.1 (Unlicense OR MIT, manifest MSRV 1.70). No bundled/system/lazy timezone loading features are enabled. The reviewed core unsafe path constructs strings from its validated fixed buffer; the unchecked date constructor is commented out. This is source inspection, not an independent audit. Resolved notices are retained with `tools/dependency_notices.py`; Cargo.lock also records optional, unbuilt packages.

R2 validation: all 19 `tools/verify.py --benchmark` stages passed, including the new 1,022-case date matrix, 58 ordinary Rust tests, existing corpora, database bytes and mutation replay. Warm median runtime ratio was 1.82 with matching output checksum. The component split is three exact matches, one known CDF discrepancy and one exact explicit-backreference error; the last two are still compatibility gaps. Additional research confirms GNU literal escaped `d`/`0` and backslash-in-bracket semantics differ from the current regular backend; these are R6 front-end cases, not covered by the old regular-pattern totals. Year-edge tests and standard-time FILETIME vectors are now ordinary passing tests.
