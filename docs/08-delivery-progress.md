# Remaining-work delivery progress

Tracking: https://github.com/JLuethe/rfile/issues/3. Specification: [remaining work](07-remaining-work-spec.md). Spec PR #2 is merged. This ledger distinguishes implemented evidence from release gates.

| Milestone | Current state | Evidence / next gate |
|---|---|---|
| R0 new discrepancy coverage | merged in PR #4 | `tools/component_compare.py`: 5 comparisons, 3 exact and 2 exact-baseline known-open discrepancies; changed results/errors fail rather than inherit a broad ignore. Two Rust controls pass; two explicitly ignored regressions retain the missing expected behavior. |
| R1 shared safety contracts | merged in PR #5 | positional IO and tracked buffers, shared work/IO/memory budgets, cancellation/deadlines, typed resource errors, semantic profiles and machine-readable capabilities; 13 new boundary tests |
| R2 immediate semantic repairs | merged in PR #6 | checked immutable TZif/POSIX contexts; local/UTC families and explicit corrected FILETIME; decoded regex lexer rejects backreferences and Rust-only groups |
| R3 CDF | merged in PR #7 | shared positional sector/mini/FAT/DIFAT traversal, typed directory/property/catalog evidence, corrected rendering, pinned fallback and generated fixtures |
| R4 ELF/core | merged in PR #8 | OS/version/build ID/PaX/memtag/core/AUXV/Solaris, positional metadata and bounded stream spooling; 5,461 exact component comparisons |
| R5 codecs | merged in PR #9 | opt-in bounded adapters, independent fixtures and per-codec fuzz targets; details below |
| R6 full semantics | regex merged in PR #10; locale/display merged in PR #11, hosted CI passed | R6c implemented locally: 25 stages passed, 15,447 exact float and 2,850 exact date comparisons; full native history/custom-zone closure remains |
| R7 sandbox | implementation on feat/linux-confinement; final rerun pending | ordered CLI, strict descriptors, typed kernel policy, child denial tests; see current R7 checkpoint below |
| R8 C ABI | pending | source diagnostics/list prerequisite and all 19 ABI functions/client contracts |
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

## R3 CDF implementation and distinctions

`Classification.compound_document` now retains directory record indexes/raw UTF16 names, tree links, CLSIDs, timestamps, stream locations, ordered property sets (FMTIDs/system/version), duplicate property IDs, exact floating bits, vector values, opaque bounded types, catalog names/header bytes, diagnostics and the failure stage. `cdf::inspect` accepts a caller-owned positional `InputView`; `parse_property_stream` exposes the bounded property parser for independent testing. Only detector-required streams are read; this is not document extraction or editing.

File/descriptor inputs can fetch sectors beyond the initial sample. Buffer and sequential-reader behavior remains limited to the provided prefix, matching their available evidence. Sector arithmetic, sentinel checks, finite chains, revisits, short reads, directory graph validation and all nested allocations consume the same budget. Limits/IO/cancellation errors propagate; malformed CDF stages retain a descriptive fallback. The pin ignores the header FAT count and stops its inline/extension lists on signed sentinel values; Pinned548 now follows that observed rule, while corrected mode requires the count to agree. Duplicate FAT references remain a checked malformed rejection.

Pinned548 closes the original missing `Cannot read section info` suffix without following the C parser's invalid stack-derived pointer. CorrectedV1 reads valid properties and renders the explicit standard labels, separates additional FMTIDs, decodes supported UTF16/1252/UTF8/ASCII, recognizes application/installer MIME precedence and bounded catalogs. Unsupported codepages are explicit diagnostics with original property bytes retained. Unknown/byref/array properties are opaque evidence; they are never pointer dereferences or invented zero values. Corrected summary/catalog output is an intentional enhancement, not primary-oracle equality. Catalog-only recognition and support for its 12-byte header are separately tested; primary source catalog dispatch appears unreachable under the pinned flag branches and is not claimed as an observed compatibility path.

The checked wire reader corrects the original specification's overbroad vector-padding statement: packed I16/BOOL vectors pad at their end, while variant/string elements follow their type's layout. Source: [Microsoft MS-OLEPS TypedPropertyValue](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-oleps/f122b9d7-e5cf-4484-8466-83f6fd94b3cc).

Evidence includes 24 small generated fixtures plus an 8 MiB/DIFAT case, CLI and buffer/path API comparisons, 14 focused Rust tests, prefix/length mutations, two new buildable sanitizer fuzz targets, ten exact scalar comparisons against the separately built two-line pointer-origin-patched C oracle, and four scalar documents independently read by olefile 0.47. `reports/cdf-pointer-origin.patch` and its hash distinguish the auxiliary oracle from the unchanged primary pin. Olefile's LPWSTR-vector path fails on the larger fixture; `reports/cdf-independent.json` records that limitation instead of counting it as independent validation.

The Rust test for the original CDF discrepancy is now enabled and passes. The corpus tracks the excessive-vector limit as a deliberate rejection rather than hiding it as exact compatibility. No 24 CPU-hour campaign or independent security review has yet passed. Full CDF release certification still requires the component's complete branch/capability matrix and final-candidate security gates; this implementation is not by itself that certification.

R3 review also centralized positional-read attempt accounting in the range driver. Custom adapters now provide `read_at_unaccounted`; the checked default method and core range driver charge before calling it, so repeated adapter interruptions cannot bypass IO limits. The adapter-interruption regression and an adjacent-unknown-property bounds regression are ordinary tests. Initial CDF sanitizer smoke reports are preserved separately from the replay after these changes. User scope was clarified to the five implementation steps through sandbox/C ABI; sustained campaigns, independent review and release qualification remain future gates.

R3 final validation: all 20 verification stages passed, including 74 ordinary Rust tests, 400 CDF comparisons (385 exact plus 15 exact named excessive-vector limit results, no unclassified differences), the earlier compiler/corpus/CLI/date checks, deterministic mutation replay and equal benchmark checksum (1.82 warm median runtime ratio). Fresh ASan runs after the final bounds/IO repairs executed 218,451 container cases and 975,213 property cases, consuming 26.52 and 30.87 measured CPU seconds respectively with no reported failure. Both reports say `smoke-only` and `completed_gate=false`; the initial pre-repair-accounting smoke reports remain separate. No sustained or independent-review claim is made.

## R4 ELF/core implementation and verification

The seekable evaluator now handles the pinned OS-version formulas, GNU/Go IDs and duplicate suppression rules, Android NDK strings, PaX/memtag labels, NetBSD annotations, SVR4/FreeBSD/NetBSD core process records, AUXV identities/virtual strings, Solaris hardware/software capabilities and malformed-header/note diagnostics. `Classification.elf` retains the owned suffix, executable decision, class/order/machine/type and ordered note evidence with file offsets and bounded owner bytes. `elf::inspect` accepts an InputView, enabling synthetic seekable fuzzing without attacker-selected paths. Buffer classification remains separate from descriptor enrichment. Output buffers and retained error bytes use the inspection budget; per-call state is owned and thread confined.

Observable legacy contracts preserved by explicit probes include zero alignment defaulting to four, division-based non-power-of-two alignment, repeated Go IDs, the source's Android memtag bit labels and double-colon NetBSD emulation label, raw byte strings, sticky PIE linkage state, note-before-dynamic MIME scheduling, prior-header section-name scheduling, count-limit annotations and the 1,024-byte per-printf ceiling. Oversized interpreter output returns the exact legacy diagnostic. The 51st AUXV element preserves the preceding raw ELF diagnostic and error text. `Error::raw_diagnostic` exposes authoritative bytes for this legacy error; Display remains a UTF8 convenience. Source capability tables retain notices, original order, source hashes and a reproducible generator.

CorrectedV1 resolves checked ELF extended counts through section zero and examines the current section name. These are named profile changes; the pinned profile retains raw u16 counts and the prior-header name quirk. Malformed offset arithmetic and aggregate memory/IO/cancellation limits remain explicit safety boundaries. Rust backend IO errors preserve typed IO information; exhaustive host-specific injected-failure prose is not certified. Only Linux/x86_64/glibc is currently executed; no other-platform certification or complete security claim follows from this milestone.

ELF stdin/Read inputs now use bounded in-memory positional spooling rather than losing metadata. `Limits.stream_bytes` / `-P spool=N` defaults to 64 MiB, hard maximum 256 MiB, and shares aggregate work/IO/memory limits. Inputs beyond this cap fail explicitly; the native implementation's potentially unbounded temporary-file spool is not reproduced. The ordinary softmagic sample remains `Limits.bytes`; no-ELF/annotation flags bypass spooling. Read input is consumed, while borrowed File classification restores its initial cursor.

Validation: 188 independent synthetic ELF fixtures cover 32/64-bit and LE/BE layouts, all named note families, raw modes, core/AUXV, duplicates, malformed/count boundaries, every source capability bit and selected profile changes. A separate sparse >4 GiB case checks large offsets without materializing its hole. The component suite has 5,461 exact comparisons across path/buffer/borrowed-descriptor APIs, CLI and stdin, including exact error bytes/statuses. All 21 full-project verification stages passed with 86 ordinary tests and two separately recorded ignored tests, exact v21 bytes, old corpora, date/CDF checks and mutation replay. Benchmark checksum matches; warm median Rust/C runtime ratio 1.80. ASan with instrumented std executed 445,255 seekable ELF cases in 28.62 measured CPU seconds without a reported failure; the binary/source hashes match the checked candidate. This is smoke evidence, not a 24-hour campaign or independent review.

## R5 bounded codec delivery

`src/codecs/` now supplies a typed registry/probe/decoder result and `Classification.compression`. Gzip/zlib remain defaults; the `extended-codecs` umbrella or individual features opt into bzip2, XZ/LZMA/lzip, Zstd, modern LZ4, UNIX compress and first-entry stored/deflate ZIP. Production uses neither native codec bindings nor helpers. Default activation remains blocked on the declared sustained security gates; compiled capabilities are reported accurately.

The independent bzip2/LZW decoders, low-level ZIP deflate driver and preflighted dependency adapters share output/work/window/member/memory/cancellation limits. XZ has an independent envelope/index validator before backend allocation. Checksums are checked where present; CHECK_NONE/LZMA/.Z retain absent integrity. CorrectedV1 joins selected native-format members; Pinned548 preserves first-member behavior. Full adapter decisions, allocation envelopes, backend unsafe inventory and remaining historical/descriptor/dictionary gaps are in [compression](remaining/03-compression.md#r5-implemented-adapter-decisions) and `reports/codec-dependencies.json`.

Validation: 82 independently encoded fixtures cover both profiles, empty/text/binary/random/runs, seven concatenation families, UNIX compress widths, XZ checks/filter chains/multiple blocks, LZ4 linked/check variants and ZIP64 descriptors. Eight new test groups cover payload equality, incomplete first-member prefixes, corruption, feature registry, shared caps, reservation release, cancellation/deadlines and skippable/trailing policies. Default/all-feature tests and Clippy pass; each of six optional features compiles independently. Rust 1.87 checks all targets/features, and hosted CI now includes that minimum plus default/all-feature tests.

`tools/verify.py --all-features --benchmark` passed all 22 stages: 94 ordinary debug/release tests, exact database bytes, 103 original fixtures, existing 22,544 broad / 11,000 flag / 10,875 rule comparisons, 5,461 ELF / 400 CDF / 1,022 date comparisons, CLI, mutation replay and linkage checks. The new compression matrix has 3,268 comparisons: 2,444 exact and 824 individually frozen intentional decisions (264 `.Z` helper dispatch, 520 ZIP helper diagnostics, 40 valid-empty LZ4), zero unexpected. Lzip is validated independently and explicitly excluded from the capability-matched native oracle total. Both profiles intentionally avoid duplicating helper-dependent `.Z` output, inspect valid first ZIP contents and accept valid empty LZ4. Malformed codec errors remain typed Rust errors rather than a claim of exact native-library prose.

Ten ASan targets with instrumented std executed 1,412,219 cases over 202.63 measured CPU seconds with no reported failure; final source and instrumented-binary hashes were rechecked. This is smoke only. The 20-case warm decoder benchmark validates all payloads and records median/p95 plus tracked memory; its supervised process peaked at 16,676 KiB RSS. It has no unfair raw-codec/native-classifier runtime ratio and no adversarial latency claim. The existing equivalent-output classification benchmark checksum still matches. Sustained campaigns, independent review and other-platform execution remain open.

## R6a regex increment and explicit remaining work

The original `(ab)\\1` discrepancy is now an ordinary passing regression in source and imported v21 form; the component suite now has five exact results. `regex_dialect` normalizes GNU C-byte escapes/classes and `regex_vm` retains complete capture histories under shared finite state/step/memory budgets. Regular patterns keep the automata fast path. Empty/unset captures, nested retained values, repeated references, case folding, anchors, compile expansion and profile/concurrency isolation have controls. The HIR reference marker has a nonempty sentinel so parser optimization cannot erase dynamic repetition.

Pinned548 deliberately rejects a conservative class of quantified-reference history constructs whose native glibc behavior has not been ported; CorrectedV1 explicitly enumerates those histories. This is an announced capability boundary, not a false nonmatch or a claim of full GNU fidelity. The independently reproduced unknown-escaped-lowercase/ICASE glibc quirk is retained only in the pinned front end. Both profiles still use the pinned last-byte/NUL window. See [the exact contract](remaining/04-semantics.md#r6a-delivered-regex-contract-and-remaining-fidelity-boundary).

The current matrix uses 84 pattern programs (including one separately rejected native grammar case), 83 inputs, case flags and two actual parser routes: 27,556 comparisons, 23,396 exact and 4,160 explicit unsupported results, zero unexpected. A separate source-only pathname avoids accidentally selecting its compiled neighbor. The invalid-range native compiler errors are reported separately, never counted as successful classifications. Locale display, exceptional dates/float formatting and full native quantified-history fidelity remain open; R6 is not marked complete by this increment.

Final regex validation: all 23 `tools/verify.py --all-features --benchmark`
stages passed, including 105 ordinary debug/release tests, one separately run
ignored mutation campaign, unchanged database bytes and old corpora, the five
exact component controls and existing named CLI/CDF/codec decisions. The extra
newline/word-boundary/reference-count matrix was replayed against the same
release binary after that full run. Default tests/Clippy, all-features Clippy and
Rust 1.87 checks pass. The final ASan capture target executed 3,002 cases over
27.43 measured CPU seconds without a reported failure; source and binary hashes
match the candidate. This short structured run is not a sustained gate or
independent review. The backreference expected-match regression is no longer
ignored; only the separately executed stable mutation test stays ignored in the
ordinary test invocation.

## R6b locale display and C-string output

The root is now a workspace with `rfile-platform`. Its Linux/glibc locale adapter
owns an immutable locale acquisition, uses a private thread-bound restoration
guard, and contains the inventoried unsafe ABI calls. Core detection still
forbids unsafe. Rust API defaults are portable C display/UTC; callers can provide
`DisplayContext::locale("C.UTF-8")`. CLI initialization snapshots LC_ALL, then
LC_CTYPE, then LANG, configuring only LC_CTYPE before rule parsing. It does not
mutate process locale/environment or apply locale to numeric/time categories.
C, C.UTF-8 and en_US.UTF-8 are executed; other ASCII/UTF8 locales need their own
certification, and other encodings/native platforms return explicit errors.

Filename rendering, displayed width, library result escaping and byte capture
printing are separate contracts. Invalid filename sequences reset/consume one
byte; invalid result sequences restart the entire result byte-wise. Native
low-byte escaping of nonprintable wide filename characters and width clamping
are preserved. Result escaping now occurs once after the full detection pipeline.
Formatted NULs obey C-string boundaries on later appends/final results, including
raw mode; successful empty matches stay empty, and the three-byte keep-going
separator quirk remains. Filesystem errors retain original diagnostic bytes and
OS error sources. Paths are capped at 65,536 encoded bytes before OS conversion
or diagnostic copying, with a tracked workspace reservation and work charge.

All 24 full-project stages passed with 113 ordinary debug/release tests, default
and all-feature checks, Rust 1.87, unchanged database bytes/old corpora and the
existing explicit decisions. The locale matrix has 1,660 exact CLI/API results
across C/C.UTF-8/en_US.UTF-8, including invalid/truncated UTF8, controls, combining/
wide/emoji names, lists, raw/NUL/alignment, missing/broken links, environment
precedence, whole-result fallback, compression with NUL/keep-going, empty
matches, MIME and path/buffer/descriptor entry points. Reports retain raw
channels/status, compiled-rule and locale-data hashes. Error comparisons in the
ELF harness now use authoritative diagnostic bytes. The final locale ASan run
executed 689,299 cases over 30.99 measured CPU seconds, no reported failure;
source/binary identities match. This is smoke only, and does not instrument or
prove the host C library itself. `reports/unsafe.json` inventories 15 first-party
unsafe declarations/blocks (including tests) within the locale adapter. The
[platform contract](../crates/rfile-platform/README.md) states the invariants and
trusted-host assumptions. No independent audit or sustained gate is claimed.


## R6c formatter/date increment

The typed formatter now respects accepted precision instead of clamping at 64,
places zero fill after signs, preserves alternate decimal points and implements
the native alternate-g carry quirk. Nonfinite values and integer-precision/character
padding retain spaces. Pinned float fmtcheck/field/output errors preserve diagnostic
bytes; CorrectedV1 supports uppercase formats. Native source acceptance and
compiled-description runtime behavior have separate routes in the float matrix.

Explicit POSIX timezone rules are validated through Jiff then held in a bounded
immutable representation for native UTC-year/pre-1970 transition semantics.
Corrected mode retains Jiff's recurring calendar behavior. Legacy FILETIME handles
the bounded native standard-time probe search and one-hour fallback for unusually
short standard-time periods. The upper-year 400-year shift remains checked.
No process locale/TZ mutation, time FFI, native printf or new dependency was added.

All 25 full-project verification stages passed with 120 ordinary debug/release
tests, unchanged compiled database bytes, old matrices and named differences.
The expanded float matrix has 15,447 exact results through source/v21 routes;
dates have 2,850 exact results across 15 timezone configurations. Default tests,
Clippy and Rust 1.87 pass. Warm benchmark checksum matches; runtime ratio is 1.55.
The new time_context fuzz target brings the total to 23. All-bin check/formatting
were repeated after adding it, separately recorded in r6c-additional-checks.json.
Final ASan smoke: 223,077 formatter cases (30.31 CPU seconds) and 662,838 timezone
cases (30.31 CPU seconds), no reported failure; source/binary identities match.
These are smoke-only reports, not sustained gates or an independent review.

Full native quantified-capture fidelity, exotic/custom TZif histories, caller
rounding/locale state and other platforms remain explicit restrictions or future
qualification. R6c closes this measured increment, not every SEM requirement.


## R7 implementation checkpoint — 2026-10-25

The existing SBX specification was reviewed and refined, not replaced. The
optional `sandbox` feature implements x86_64 Linux/glibc CLI auto/strict/off modes,
actual stderr status, baseline-before-resource-parsing and bounded strict input
preparation. `-f` frozen-cookie/output ordering remains explicit; strict rejects
streaming filename lists and mixed compile/classification actions. Compilation
uses private staging and descriptor-relative publication on sandbox builds.

The adapter uses landlock 0.4.7, seccompiler 0.5.0 (JSON disabled) and rustix 1.1.5
filesystem APIs. Fixed Landlock V5 rights are reported as 0xffff, even on the
observed ABI 10 host. The core remains `forbid(unsafe_code)`; the inventoried
first-party unsafe boundary is 23 lexical entries including tests. Inherited
extra FD authority is denied rather than unsafely closing unknown Rust owners.
Kernel and strict-input state are thread-bound; ordinary library calls install
nothing. No feature default, system installation or release is changed.

Focused tests cover mode ordering, actual status, stdin, file lists, later option
errors, symlinks, missing paths, timestamp restoration and compile replacement.
Eight disposable kernel-child scenarios cover read-only/preopened/compile,
inherited FD denial, unavailable and denied setup, deleted/replaced file identity,
and gaps between approved descriptor ranges. No real unsupported-host skip is
counted as certification. Pure injected ENOSYS/EPERM cases are labeled tests.

The first full all-feature run passed 26 stages and 126 ordinary debug/release
tests. Its sandbox matrix passed 104 batches: 5,760 operand comparisons across
411 fixtures, covering every compiled codec, MIME, annotations and keep-going.
An additional 128-input boundary probe found filter-size overflow; exact adjacent
FD range compression fixed it without admitting holes, and the at-cap `-p` test
passes. **Final full rerun after that change is pending at this checkpoint.**
MSRV 1.87 passes. A prior simultaneous default/all-feature test invocation replaced
the CLI executable mid-test; future feature runs are serialized or isolated.

Remaining R7 qualification includes native syscall tracing, additional kernel/
container/architecture environments and independent review. Historical reports
are tied to their binary hashes. R8 source diagnostics/list compatibility and all
19 C functions are still next; sustained fuzzing and security review are not
completed by these focused tests.
